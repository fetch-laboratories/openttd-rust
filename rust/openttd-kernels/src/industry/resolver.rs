/* This file is part of OpenTTD and is licensed under the GNU General Public License version 2. */
//! Industry and industry-tile resolvers. Contexts are stack values; industry state and the
//! ordered registries are read directly, and Action2, town and storage calls can reenter.
use super::{
    Builder, ConstructionServices, GenerationServices, Industry, LifecycleServices, SpecHandle,
    TileServices, TileSpec, TownHandle,
};
use crate::history_type::{LAST_MONTH, THIS_MONTH};
use crate::industry_type::{
    BUILT_ON_WATER, INVALID_INDUSTRY, INVALID_INDUSTRYTILE, IT_INVALID, NEW_INDUSTRYTILEOFFSET,
    NUM_INDUSTRYTYPES, PROD_MULTI_HANDLING,
};
use crate::newgrf_callbacks::{
    CALLBACK_FAILED, CBID_INDUSTRY_LOCATION, CBID_INDUSTRY_PROBABILITY, CBID_INDUSTRY_REFUSE_CARGO,
    INDUSTRY_PROBABILITY, INDUSTRY_PRODUCTION_256_TICKS, INDUSTRY_PRODUCTION_CARGO_ARRIVAL,
    INDUSTRY_REFUSE_CARGO,
};
use crate::newgrf_spritegroup::{VSG_SCOPE_PARENT, VSG_SCOPE_SELF};
use crate::tile_type::{INVALID_TILE, MP_INDUSTRY};
use crate::timer_game_common::{DAYS_TILL_ORIGINAL_BASE_YEAR, ORIGINAL_BASE_YEAR};
use crate::witness;
use std::{ffi::c_void, ptr};

#[derive(Clone, Copy)]
#[repr(C)]
pub struct ResolverContext {
    pub owner: *mut Industry,
    pub industry: *mut c_void,
    pub town: *mut TownHandle,
    pub tile: u32,
    pub random_bits: u32,
    pub id: u16,
    pub kind: u8,
}
#[repr(C)]
pub struct ResolverRef {
    pub owner: *mut Industry,
    pub town: *mut TownHandle,
}
#[repr(C)]
pub struct FounderInfo {
    pub colours: u8,
    pub is_ai: bool,
}
#[repr(C)]
pub struct ResolverServices {
    pub shared: *const crate::services::Services,
    pub generation: *const GenerationServices,
    pub construction: *const ConstructionServices,
    pub lifecycle: *const LifecycleServices,
    pub tile: *const TileServices,
    pub get: unsafe extern "C" fn(u16) -> ResolverRef,
    pub spec_grfid: unsafe extern "C" fn(*const SpecHandle) -> u32,
    pub spec_local_id: unsafe extern "C" fn(*const SpecHandle) -> u16,
    pub root: unsafe extern "C" fn(*const SpecHandle, bool) -> *const c_void,
    pub tile_root: unsafe extern "C" fn(*const TileSpec, bool) -> *const c_void,
    pub tile_local_id: unsafe extern "C" fn(*const TileSpec) -> u16,
    pub tile_override: unsafe extern "C" fn(*const TileSpec) -> u16,
    pub tile_grfid: unsafe extern "C" fn(*const TileSpec) -> u32,
    pub tile_has_groups: unsafe extern "C" fn(*const TileSpec) -> bool,
    pub tile_substitute: unsafe extern "C" fn(*const TileSpec) -> u16,
    pub clean_gfx: unsafe extern "C" fn(u32) -> u16,
    pub nearby: unsafe extern "C" fn(u8, u32, bool) -> u32,
    pub nearby_info: unsafe extern "C" fn(u32, bool) -> u32,
    pub terrain: unsafe extern "C" fn(u32) -> u32,
    pub town_zone: unsafe extern "C" fn(*const TownHandle, u32) -> u8,
    pub water_distance: unsafe extern "C" fn(u32, bool) -> u32,
    pub town_tile: unsafe extern "C" fn(*const crate::services::TownHandle) -> u32,
    pub town_id: unsafe extern "C" fn(*const crate::services::TownHandle) -> u16,
    pub founder_info: unsafe extern "C" fn(u8) -> FounderInfo,
    pub grfid: unsafe extern "C" fn(*const c_void) -> u32,
    pub version: unsafe extern "C" fn(*const c_void) -> u8,
    pub callback_id: unsafe extern "C" fn(*const c_void) -> u16,
    pub cargo_translation: unsafe extern "C" fn(*const c_void, u8) -> u8,
    pub cargo_map: unsafe extern "C" fn(*const SpecHandle, u8) -> u8,
    pub badges: unsafe extern "C" fn(*const c_void, *const SpecHandle, u32) -> u32,
    pub tile_badges: unsafe extern "C" fn(*const c_void, *const TileSpec, u32) -> u32,
    pub storage_owner:
        unsafe extern "C" fn(*mut c_void) -> *mut crate::newgrf_storage::Payload<256>,
    pub unhandled: unsafe extern "C" fn(u8, bool, bool, u16),
    pub pointer_magic: unsafe extern "C" fn(),
    pub base_scope: unsafe extern "C" fn(*mut c_void, u8, u8) -> *mut c_void,
    pub existing_town: unsafe extern "C" fn(*mut c_void) -> *mut c_void,
    pub make_town: unsafe extern "C" fn(*mut c_void, *mut TownHandle, bool) -> *mut c_void,
    pub callback: unsafe extern "C" fn(u16, u32, u32, *mut c_void, u8, u32) -> u16,
    pub boolean: unsafe extern "C" fn(*const SpecHandle, u16, u16) -> bool,
    pub callback_error: unsafe extern "C" fn(*const SpecHandle, u16, u16),
}
impl ResolverServices {
    fn shared(&self) -> &crate::services::Services {
        unsafe { &*self.shared }
    }
    fn generation(&self) -> &GenerationServices {
        unsafe { &*self.generation }
    }
    fn construction(&self) -> &ConstructionServices {
        unsafe { &*self.construction }
    }
    fn lifecycle(&self) -> &LifecycleServices {
        unsafe { &*self.lifecycle }
    }
    fn tile(&self) -> &TileServices {
        unsafe { &*self.tile }
    }
    fn map(&self) -> crate::map_func::Map {
        self.shared().map()
    }
    fn is_industry(&self, tile: u32) -> bool {
        (self.shared().tile_type)(tile) == MP_INDUSTRY
    }
    fn belongs(&self, tile: u32, id: u16) -> bool {
        unsafe { (self.lifecycle().belongs)(tile, id) }
    }
    fn spec(&self, kind: u8) -> *const SpecHandle {
        unsafe { (self.generation().spec)(kind) }
    }
    fn behaviour(&self, spec: *const SpecHandle) -> u32 {
        unsafe { (self.generation().spec_behaviour)(spec) }
    }
}

fn map_type(kind: u8, grfid: u32, _s: &ResolverServices) -> u8 {
    if kind == IT_INVALID {
        return IT_INVALID;
    }
    if kind & 0x80 == 0 {
        return kind & 0x7f;
    }
    (unsafe { crate::override_manager::industry_id(u16::from(kind & 0x7f), grfid) }) as u8
}
fn id_at_offset(tile: u32, id: u16, grfid: u32, s: &ResolverServices) -> u32 {
    if !s.belongs(tile, id) {
        return 0xffff;
    }
    let gfx = unsafe { (s.clean_gfx)(tile) };
    let spec = unsafe { (s.tile().spec)(gfx) };
    if gfx < NEW_INDUSTRYTILEOFFSET {
        let override_id = unsafe { (s.tile_override)(spec) };
        if override_id == INVALID_INDUSTRYTILE {
            return 0xff00 | u32::from(gfx);
        }
        let other = unsafe { (s.tile().spec)(override_id) };
        return if unsafe { (s.tile_grfid)(other) } == grfid {
            u32::from(unsafe { (s.tile_local_id)(other) })
        } else {
            0xfffe
        };
    }
    if unsafe { (s.tile_has_groups)(spec) } {
        if unsafe { (s.tile_grfid)(spec) } == grfid {
            u32::from(unsafe { (s.tile_local_id)(spec) })
        } else {
            0xfffe
        }
    } else {
        0xff00 | u32::from(unsafe { (s.tile_substitute)(spec) })
    }
}
fn nearby_information(
    parameter: u8,
    mut tile: u32,
    id: u16,
    signed: bool,
    version8: bool,
    s: &ResolverServices,
) -> u32 {
    if parameter != 0 {
        tile = unsafe { (s.nearby)(parameter, tile, signed) };
    }
    let same = s.belongs(tile, id);
    (unsafe { (s.nearby_info)(tile, version8) }) | (u32::from(same) << 8)
}
pub(crate) fn relative_position(tile: u32, north: u32, map: crate::map_func::Map) -> u32 {
    let x = map.tile_x(tile).wrapping_sub(map.tile_x(north)) as u8;
    let y = map.tile_y(tile).wrapping_sub(map.tile_y(north)) as u8;
    (u32::from(y & 15) << 20) | (u32::from(x & 15) << 16) | (u32::from(y) << 8) | u32::from(x)
}
unsafe fn registry_count(builder: *const Builder, kind: u8) -> usize {
    unsafe { (&(*builder).registry)[usize::from(kind)].len() }
}
unsafe fn registry_id(builder: *const Builder, kind: u8, n: usize) -> u16 {
    unsafe { (&(*builder).registry)[usize::from(kind)][n] }
}
fn closest(
    tile: u32,
    kind: u8,
    current: &ResolverContext,
    builder: *const Builder,
    s: &ResolverServices,
    map: crate::map_func::Map,
) -> u32 {
    let mut best = u32::MAX;
    for n in 0..unsafe { registry_count(builder, kind) } {
        let id = unsafe { registry_id(builder, kind, n) };
        if id == current.id {
            continue;
        }
        let other = unsafe { (s.get)(id) };
        let other_tile = unsafe { (*other.owner).metadata.tile };
        best = best.min(map.distance_manhattan(tile, other_tile));
    }
    best
}
fn count_and_distance(
    mut parameter: u8,
    layout: u8,
    town: bool,
    current: &ResolverContext,
    builder: *const Builder,
    s: &ResolverServices,
) -> u32 {
    let mut grfid = unsafe { crate::newgrf_storage::openttd_rust_storage_temp_get(0x100) } as u32;
    let kind = if grfid == 0 {
        parameter
    } else {
        if grfid == u32::MAX {
            grfid = unsafe { (s.spec_grfid)(s.spec((*current.owner).metadata.kind)) };
        }
        parameter |= 0x80;
        map_type(parameter, grfid, s)
    };
    if kind >= NUM_INDUSTRYTYPES {
        return 0xffff;
    }
    let map = s.map();
    let tile = unsafe { (*current.owner).metadata.tile };
    let mut count: u8 = 0;
    let mut distance = u32::MAX;
    if layout == 0 && !town {
        witness::hit(witness::INDUSTRY_RESOLVER_COUNT);
        distance = closest(tile, kind, current, builder, s, map);
        count = unsafe { registry_count(builder, kind) }.min(255) as u8;
    } else {
        witness::hit(witness::INDUSTRY_RESOLVER_FILTERED_COUNT);
        for n in 0..unsafe { registry_count(builder, kind) } {
            let id = unsafe { registry_id(builder, kind, n) };
            if id == current.id {
                continue;
            }
            let other = unsafe { (s.get)(id) };
            if (layout == 0 || unsafe { (*other.owner).metadata.selected_layout } == layout)
                && (!town || other.town == current.town)
            {
                distance = distance
                    .min(map.distance_manhattan(tile, unsafe { (*other.owner).metadata.tile }));
                count = count.wrapping_add(1);
            }
        }
    }
    (u32::from(count) << 16) | (distance & 0xffff)
}
fn unavailable(
    variable: u8,
    available: *mut bool,
    tile: bool,
    missing: bool,
    callback: u16,
    s: &ResolverServices,
) -> u32 {
    unsafe {
        (s.unhandled)(variable, tile, missing, callback);
        *available = false;
    }
    u32::MAX
}
fn town_tile(town: *const TownHandle, s: &ResolverServices) -> u32 {
    unsafe { (s.town_tile)(town.cast()) }
}
fn town_id(town: *const TownHandle, s: &ResolverServices) -> u32 {
    u32::from(unsafe { (s.town_id)(town.cast()) })
}
fn variable(
    c: &ResolverContext,
    ro: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
    builder: *const Builder,
    s: &ResolverServices,
) -> u32 {
    witness::hit(witness::INDUSTRY_RESOLVER_VARIABLE);
    let callback = unsafe { (s.callback_id)(ro) };
    let i = c.owner;
    if callback == CBID_INDUSTRY_LOCATION {
        witness::hit(witness::INDUSTRY_RESOLVER_LOCATION);
        match variable {
            0x7a => return unsafe { (s.badges)(ro, s.spec(c.kind), parameter) },
            0x80 => return c.tile,
            0x81 => return (c.tile >> 8) & 255,
            0x82 => return town_id(c.town, s),
            0x83..=0x85 => unsafe {
                (s.pointer_magic)();
            },
            0x86 => return u32::from(unsafe { (*i).metadata.selected_layout }),
            0x87 => return unsafe { (s.terrain)(c.tile) },
            0x88 => return u32::from(unsafe { (s.town_zone)(c.town, c.tile) }),
            0x89 => {
                return s
                    .map()
                    .distance_manhattan(town_tile(c.town, s), c.tile)
                    .min(255);
            }
            0x8a => {
                return (s.shared().tile_z)(c.tile)
                    .wrapping_mul(if unsafe { (s.version)(ro) } >= 8 {
                        1
                    } else {
                        crate::tile_type::TILE_HEIGHT as i32
                    })
                    .clamp(0, 255) as u32;
            }
            0x8b => {
                return unsafe {
                    (s.water_distance)(
                        c.tile,
                        s.behaviour(s.spec((*i).metadata.kind)) & BUILT_ON_WATER == 0,
                    )
                };
            }
            0x8d => {
                return s
                    .map()
                    .distance_square(town_tile(c.town, s), c.tile)
                    .min(65535);
            }
            0x8f => return c.random_bits,
            _ => {}
        }
    }
    let spec = s.spec(c.kind);
    if i.is_null() {
        return unavailable(variable, available, false, true, callback, s);
    }
    match variable {
        0x40..=0x42 => {
            let callbacks = unsafe { (s.construction().callback_mask)(spec) };
            if callbacks & (INDUSTRY_PRODUCTION_CARGO_ARRIVAL | INDUSTRY_PRODUCTION_256_TICKS) == 0
            {
                return 0;
            }
            let multiplier = if s.behaviour(spec) & PROD_MULTI_HANDLING != 0 {
                let level = unsafe { (*i).fields.prod_level };
                if level == 0 {
                    return 0;
                }
                level
            } else {
                1
            };
            u32::from(
                unsafe { (*super::get_accepted(i, usize::from(variable - 0x40))).waiting }
                    / u16::from(multiplier),
            )
        }
        0x43 if c.tile != INVALID_TILE => unsafe {
            (s.water_distance)(c.tile, s.behaviour(spec) & BUILT_ON_WATER == 0)
        },
        0x44 => u32::from(unsafe { (*i).metadata.selected_layout }),
        0x45 => {
            let founder = unsafe { (*i).metadata.founder };
            {
                let info = unsafe { (s.founder_info)(founder) };
                u32::from(founder) | (u32::from(info.is_ai) << 16) | (u32::from(info.colours) << 24)
            }
        }
        0x46 => (unsafe { (*i).metadata.construction_date }) as u32,
        0x47 => u32::from(unsafe { (*i).fields.ctlflags }),
        0x60 => id_at_offset(
            unsafe { (s.nearby)(parameter as u8, (*i).metadata.tile, false) },
            c.id,
            unsafe { (s.grfid)(ro) },
            s,
        ),
        0x61 if c.tile != INVALID_TILE => {
            let tile = unsafe { (s.nearby)(parameter as u8, c.tile, false) };
            if s.belongs(tile, c.id) {
                u32::from(unsafe { (s.lifecycle().random_bits)(tile) })
            } else {
                0
            }
        }
        0x62 if c.tile != INVALID_TILE => nearby_information(
            parameter as u8,
            c.tile,
            INVALID_INDUSTRY,
            false,
            unsafe { (s.version)(ro) } >= 8,
            s,
        ),
        0x63 if c.tile != INVALID_TILE => {
            let tile = unsafe { (s.nearby)(parameter as u8, c.tile, false) };
            if s.belongs(tile, c.id) {
                u32::from(unsafe { (s.tile().frame)(tile) })
            } else {
                u32::MAX
            }
        }
        0x64 if c.tile != INVALID_TILE => {
            let kind = map_type(parameter as u8, unsafe { (s.spec_grfid)(spec) }, s);
            if kind >= NUM_INDUSTRYTYPES {
                u32::MAX
            } else {
                closest(c.tile, kind, c, builder, s, s.map())
            }
        }
        0x65 if c.tile != INVALID_TILE => {
            let tile = unsafe { (s.nearby)(parameter as u8, c.tile, true) };
            (u32::from(unsafe { (s.town_zone)(c.town, tile) }) << 16)
                | s.map()
                    .distance_manhattan(tile, town_tile(c.town, s))
                    .min(65535)
        }
        0x66 if c.tile != INVALID_TILE => {
            let tile = unsafe { (s.nearby)(parameter as u8, c.tile, true) };
            s.map().distance_square(tile, town_tile(c.town, s))
        }
        0x67 | 0x68 => {
            let reg = if variable == 0x68 {
                (unsafe { crate::newgrf_storage::openttd_rust_storage_temp_get(0x101) }) as u32
            } else {
                0
            };
            count_and_distance(parameter as u8, reg as u8, reg & 0x100 != 0, c, builder, s)
        }
        0x69..=0x6d | 0x70 | 0x71 => {
            let cargo = unsafe { (s.cargo_translation)(ro, parameter as u8) };
            let Some(slot) = (unsafe { super::get_cargo_produced(i, cargo) }) else {
                return 0;
            };
            let p = unsafe { &*super::produced(i, slot) };
            match variable {
                0x69 => u32::from(p.waiting),
                0x6a => u32::from(p.history[usize::from(THIS_MONTH)].first),
                0x6b => u32::from(p.history[usize::from(THIS_MONTH)].second),
                0x6c => u32::from(p.history[usize::from(LAST_MONTH)].first),
                0x6d => u32::from(p.history[usize::from(LAST_MONTH)].second),
                0x70 => u32::from(p.rate),
                _ => u32::from(p.history[usize::from(LAST_MONTH)].pct_transported()),
            }
        }
        0x6e | 0x6f => {
            let cargo = unsafe { (s.cargo_translation)(ro, parameter as u8) };
            let Some(slot) = (unsafe { super::get_cargo_accepted(i, cargo) }) else {
                return 0;
            };
            let a = unsafe { &*super::accepted(i, slot) };
            if variable == 0x6e {
                a.last_accepted as u32
            } else {
                u32::from(a.waiting)
            }
        }
        0x7a => unsafe { (s.badges)(ro, s.spec(c.kind), parameter) },
        0x7c => {
            // Resolve the native shell association once, then read the canonical
            // Rust payload directly. Temporary invalid-ID scopes can read PSA;
            // the invalid-ID gate belongs only to the storage write operation.
            let owner = unsafe { (s.storage_owner)(c.industry) };
            if owner.is_null() {
                0
            } else {
                (unsafe { crate::newgrf_storage::openttd_rust_storage256_get(owner, parameter) })
                    as u32
            }
        }
        0x80 => unsafe { (*i).metadata.tile },
        0x81 => (unsafe { (*i).metadata.tile } >> 8) & 255,
        0x82 => town_id(c.town, s),
        0x83..=0x85 => {
            unsafe {
                (s.pointer_magic)();
            }
            unavailable(variable, available, false, false, callback, s)
        }
        0x86 => u32::from(unsafe { (*i).metadata.width }),
        0x87 => u32::from(unsafe { (*i).metadata.height }),
        0x88 | 0x89 => {
            u32::from(unsafe { (*super::get_produced(i, usize::from(variable - 0x88))).cargo })
        }
        0x8a..=0x8d => {
            let p = unsafe { super::get_produced(i, usize::from((variable - 0x8a) / 2)) };
            let value = u32::from(unsafe { (*p).waiting });
            if variable & 1 != 0 {
                (value >> 8) & 255
            } else {
                value
            }
        }
        0x8e | 0x8f => {
            u32::from(unsafe { (*super::get_produced(i, usize::from(variable - 0x8e))).rate })
        }
        0x90..=0x92 => {
            u32::from(unsafe { (*super::get_accepted(i, usize::from(variable - 0x90))).cargo })
        }
        0x93 => u32::from(unsafe { (*i).fields.prod_level }),
        0x94..=0x9b | 0x9e..=0xa5 => {
            let (base, month) = if variable < 0x9c {
                (0x94, THIS_MONTH)
            } else {
                (0x9e, LAST_MONTH)
            };
            let offset = variable - base;
            let p = unsafe { super::get_produced(i, usize::from((offset % 4) / 2)) };
            let history = unsafe { (*p).history[usize::from(month)] };
            let value = u32::from(if offset < 4 {
                history.first
            } else {
                history.second
            });
            if offset & 1 != 0 {
                (value >> 8) & 255
            } else {
                value
            }
        }
        0x9c | 0x9d => u32::from(unsafe {
            (*super::get_produced(i, usize::from(variable - 0x9c))).history[usize::from(LAST_MONTH)]
                .pct_transported()
        }),
        0xa6 => u32::from(unsafe { (s.spec_local_id)(spec) }),
        0xa7 => u32::from(unsafe { (*i).metadata.founder }),
        0xa8 => u32::from(unsafe { (*i).metadata.random_colour }),
        0xa9 => unsafe { (*i).fields.last_prod_year }
            .wrapping_sub(ORIGINAL_BASE_YEAR)
            .clamp(0, 255) as u32,
        0xaa => u32::from(unsafe { (*i).fields.counter }),
        0xab => u32::from(unsafe { (*i).fields.counter } >> 8),
        0xac => u32::from(unsafe { (*i).fields.was_cargo_delivered }),
        0xb0 => unsafe { (*i).metadata.construction_date }
            .wrapping_sub(DAYS_TILL_ORIGINAL_BASE_YEAR)
            .clamp(0, 65535) as u32,
        0xb3 => u32::from(unsafe { (*i).metadata.construction_type }),
        0xb4 => {
            let accepted = unsafe { &(*i).accepted };
            accepted.iter().map(|a| a.last_accepted).max()
        }
        .map_or(0, |date| {
            date.wrapping_sub(DAYS_TILL_ORIGINAL_BASE_YEAR)
                .clamp(0, 65535) as u32
        }),
        _ => unavailable(variable, available, false, false, callback, s),
    }
}
fn tile_variable(
    c: &ResolverContext,
    ro: *const c_void,
    variable: u8,
    parameter: u32,
    available: *mut bool,
    s: &ResolverServices,
) -> u32 {
    witness::hit(witness::INDUSTRY_RESOLVER_TILE_VARIABLE);
    match variable {
        0x40 => {
            if s.is_industry(c.tile) {
                u32::from(unsafe { (s.tile().construction_stage)(c.tile) })
            } else {
                0
            }
        }
        0x41 => unsafe { (s.terrain)(c.tile) },
        0x42 => {
            let town = (s.shared().closest_town)(c.tile, u32::MAX);
            u32::from(unsafe { (s.town_zone)(town.cast(), c.tile) })
        }
        0x43 => relative_position(c.tile, unsafe { (*c.owner).metadata.tile }, s.map()),
        0x44 => {
            if s.is_industry(c.tile) {
                u32::from(unsafe { (s.tile().frame)(c.tile) })
            } else {
                0
            }
        }
        0x60 => nearby_information(
            parameter as u8,
            c.tile,
            c.id,
            true,
            unsafe { (s.version)(ro) } >= 8,
            s,
        ),
        0x61 => {
            let tile = unsafe { (s.nearby)(parameter as u8, c.tile, true) };
            if s.is_industry(tile)
                && unsafe { (s.tile().industry)(tile).shell.cast::<c_void>() } == c.industry
            {
                u32::from(unsafe { (s.tile().frame)(tile) })
            } else {
                u32::MAX
            }
        }
        0x62 => id_at_offset(
            unsafe { (s.nearby)(parameter as u8, c.tile, true) },
            c.id,
            unsafe { (s.grfid)(ro) },
            s,
        ),
        0x7a => {
            let gfx = unsafe { (s.tile().gfx)(c.tile) };
            unsafe { (s.tile_badges)(ro, (s.tile().spec)(gfx), parameter) }
        }
        _ => unavailable(variable, available, true, false, 0, s),
    }
}

/// # Safety
/// Live serialized invocation, resolver, builder and immutable table. No borrow
/// of canonical owner/container state spans a native callback.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_variable(
    c: *const ResolverContext,
    ro: *const c_void,
    var: u8,
    parameter: u32,
    available: *mut bool,
    builder: *mut Builder,
    s: *const ResolverServices,
) -> u32 {
    // End the native context borrow before a synchronous service can reenter.
    let c = unsafe { *c };
    variable(&c, ro, var, parameter, available, builder, unsafe { &*s })
}
/// # Safety
/// Live scope/resolver/table; original variable-specific map/industry preconditions.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_variable(
    c: *const ResolverContext,
    ro: *const c_void,
    var: u8,
    parameter: u32,
    available: *mut bool,
    s: *const ResolverServices,
) -> u32 {
    let c = unsafe { *c };
    tile_variable(&c, ro, var, parameter, available, unsafe { &*s })
}
/// # Safety
/// Live context; a non-null owner remains live through the immediate read.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_resolver_random(c: *const ResolverContext) -> u32 {
    let owner = unsafe { (*c).owner };
    if owner.is_null() {
        0
    } else {
        u32::from(unsafe { (*owner).metadata.random })
    }
}
/// # Safety
/// Original non-null industry/valid tile assertions are checked by the facade.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_random(
    c: *const ResolverContext,
    s: *const ResolverServices,
) -> u32 {
    let c = unsafe { *c };
    if c.id == INVALID_INDUSTRY {
        0
    } else {
        u32::from(unsafe { ((*(*s).lifecycle).random_bits)(c.tile) })
    }
}
/// # Safety
/// Original non-null industry/valid tile assertions are checked by the facade.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_triggers(
    c: *const ResolverContext,
    s: *const ResolverServices,
) -> u32 {
    let c = unsafe { *c };
    if c.id == INVALID_INDUSTRY {
        0
    } else {
        u32::from(unsafe { ((*(*s).lifecycle).random_triggers)(c.tile) })
    }
}
/// # Safety
/// Live context and native specs; root group lives for the resolver invocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_resolver_root(
    c: *const ResolverContext,
    s: *const ResolverServices,
) -> *const c_void {
    witness::hit(witness::INDUSTRY_RESOLVER_ROOT);
    let c = unsafe { *c };
    let s = unsafe { &*s };
    unsafe {
        (s.root)(
            s.spec(c.kind),
            !c.owner.is_null() && c.id != INVALID_INDUSTRY,
        )
    }
}
/// # Safety
/// Original non-null industry and valid gfx preconditions; stable native group.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_root(
    c: *const ResolverContext,
    gfx: u16,
    s: *const ResolverServices,
) -> *const c_void {
    witness::hit(witness::INDUSTRY_RESOLVER_TILE_ROOT);
    unsafe { ((*s).tile_root)(((*(*s).tile).spec)(gfx), (*c).id != INVALID_INDUSTRY) }
}
fn town(c: &ResolverContext, ro: *mut c_void, s: &ResolverServices) -> *mut c_void {
    witness::hit(witness::INDUSTRY_RESOLVER_TOWN);
    let existing = unsafe { (s.existing_town)(ro) };
    if !existing.is_null() {
        return existing;
    }
    let (town, readonly) = if !c.owner.is_null() {
        (c.town, c.id == INVALID_INDUSTRY)
    } else if c.tile != INVALID_TILE {
        (
            (s.shared().closest_town)(c.tile, u32::MAX)
                .cast_mut()
                .cast(),
            true,
        )
    } else {
        (ptr::null_mut(), true)
    };
    if town.is_null() {
        ptr::null_mut()
    } else {
        unsafe { (s.make_town)(ro, town, readonly) }
    }
}
/// # Safety
/// Live native industries resolver; optional town scope belongs to that stack frame.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_resolver_town(
    c: *const ResolverContext,
    ro: *mut c_void,
    s: *const ResolverServices,
) -> *mut c_void {
    let c = unsafe { *c };
    town(&c, ro, unsafe { &*s })
}
/// # Safety
/// All native scopes and resolver live for this synchronous resolution.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_resolver_scope(
    c: *const ResolverContext,
    ro: *mut c_void,
    own: *mut c_void,
    scope: u8,
    relative: u8,
    s: *const ResolverServices,
) -> *mut c_void {
    let s = unsafe { &*s };
    match scope {
        VSG_SCOPE_SELF => own,
        VSG_SCOPE_PARENT => {
            let c = unsafe { *c };
            let town = town(&c, ro, s);
            if town.is_null() {
                unsafe { (s.base_scope)(ro, scope, relative) }
            } else {
                town
            }
        }
        _ => unsafe { (s.base_scope)(ro, scope, relative) },
    }
}
/// # Safety
/// Live tile resolver and its two native virtual facades, valid table.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_scope(
    ro: *mut c_void,
    own: *mut c_void,
    parent: *mut c_void,
    scope: u8,
    relative: u8,
    s: *const ResolverServices,
) -> *mut c_void {
    match scope {
        VSG_SCOPE_SELF => own,
        VSG_SCOPE_PARENT => parent,
        _ => unsafe { ((*s).base_scope)(ro, scope, relative) },
    }
}
/// # Safety
/// Live mapping manager throughout the call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_map_type(
    kind: u8,
    grfid: u32,
    s: *const ResolverServices,
) -> u8 {
    map_type(kind, grfid, unsafe { &*s })
}
/// # Safety
/// Live specs/table, callback may synchronously reenter with its own stack context.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_probability(
    kind: u8,
    creation: u8,
    mut default: u32,
    s: *const ResolverServices,
) -> u32 {
    witness::hit(witness::INDUSTRY_RESOLVER_PROBABILITY);
    let s = unsafe { &*s };
    let spec = s.spec(kind);
    if unsafe { (s.construction().callback_mask)(spec) } & INDUSTRY_PROBABILITY != 0 {
        let result = unsafe {
            (s.callback)(
                CBID_INDUSTRY_PROBABILITY,
                0,
                u32::from(creation),
                ptr::null_mut(),
                kind,
                INVALID_TILE,
            )
        };
        if result != CALLBACK_FAILED {
            if unsafe { (s.construction().grf_version)(spec) } < 8 {
                if result != 0 {
                    default = 0;
                }
            } else if result < 0x100 {
                default = u32::from(result);
            } else if result > 0x100 {
                unsafe {
                    (s.callback_error)(spec, CBID_INDUSTRY_PROBABILITY, result);
                }
            }
        }
    }
    default
}
/// # Safety
/// Live owner and original accepted-cargo assertion, checked by the native facade.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_refuses(
    industry: *mut c_void,
    owner: *mut Industry,
    cargo: u8,
    s: *const ResolverServices,
) -> bool {
    unsafe { refuses(industry, owner, cargo, &*s) }
}
/// SAFETY: live owner/shell, caller has established an accepted cargo slot.
/// Action2 may reenter; owner observations end before every native call.
pub(crate) unsafe fn refuses(
    industry: *mut c_void,
    owner: *mut Industry,
    cargo: u8,
    s: &ResolverServices,
) -> bool {
    witness::hit(witness::INDUSTRY_RESOLVER_REFUSAL);
    let kind = unsafe { (*owner).metadata.kind };
    let spec = s.spec(kind);
    if unsafe { (s.construction().callback_mask)(spec) } & INDUSTRY_REFUSE_CARGO != 0 {
        let parameter = unsafe { (s.cargo_map)(spec, cargo) };
        let tile = unsafe { (*owner).metadata.tile };
        let result = unsafe {
            (s.callback)(
                CBID_INDUSTRY_REFUSE_CARGO,
                0,
                u32::from(parameter),
                industry,
                kind,
                tile,
            )
        };
        if result != CALLBACK_FAILED {
            return !unsafe { (s.boolean)(spec, CBID_INDUSTRY_REFUSE_CARGO, result) };
        }
    }
    false
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDIndustryFounderInfo" ["industry"] => FounderInfo { colours, is_ai, };
    "OpenTTDIndustryResolverContext" ["industry"] => ResolverContext { owner, industry, town, tile, random_bits, id, kind => type, };
    "OpenTTDIndustryResolverRef" ["industry"] => ResolverRef { owner, town, };
    "OpenTTDIndustryResolverServices" ["industry"] => ResolverServices {
        shared,
        generation,
        construction,
        lifecycle,
        tile,
        get,
        spec_grfid,
        spec_local_id,
        root,
        tile_root,
        tile_local_id,
        tile_override,
        tile_grfid,
        tile_has_groups,
        tile_substitute,
        clean_gfx,
        nearby,
        nearby_info,
        terrain,
        town_zone,
        water_distance,
        town_tile,
        town_id,
        founder_info,
        grfid,
        version,
        callback_id,
        cargo_translation,
        cargo_map,
        badges,
        tile_badges,
        storage_owner,
        unhandled,
        pointer_magic,
        base_scope,
        existing_town,
        make_town,
        callback,
        boolean,
        callback_error,    };
}
