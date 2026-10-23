/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Complete industry initialization, tile planting and nearby-station policy.
//! Native spec/layout handles are immutable. Slot/metadata accesses end before
//! every world callback; those callbacks can inspect partially initialized owners.
use super::{
    Accepted, Builder, GenerationServices, Industry, LifecycleServices, Map, Produced, Services,
    SpecHandle, accepted, fields, produced, produced_count, valid,
};
use crate::history_type::LAST_MONTH;
use crate::newgrf_callbacks::CALLBACK_FAILED;
use std::{ffi::c_void, ptr};

#[repr(C)]
pub struct TownHandle {
    _private: [u8; 0],
}
#[repr(C)]
pub struct LayoutHandle {
    _private: [u8; 0],
}
#[repr(C)]
pub struct StationHandle {
    _private: [u8; 0],
}
#[repr(C)]
pub struct CargoLists {
    pub produced: [u8; 16],
    pub accepted: [u8; 16],
}
#[repr(C)]
pub struct LayoutTile {
    pub x: i16,
    pub y: i16,
    pub gfx: u16,
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct ConstructionRequest {
    pub layout: *const LayoutHandle,
    pub town: *const TownHandle,
    pub layout_index: usize,
    pub tile: u32,
    pub size_x: u32,
    pub size_y: u32,
    pub id: u16,
    pub random_bits: u16,
    pub kind: u8,
    pub founder: u8,
}
#[repr(C)]
pub struct ConstructionServices {
    pub cargo_lists: unsafe extern "C" fn(*const SpecHandle) -> CargoLists,
    pub callback_mask: unsafe extern "C" fn(*const SpecHandle) -> u16,
    pub set_town: unsafe extern "C" fn(*mut c_void, *const TownHandle),
    pub calendar_date: unsafe extern "C" fn() -> i32,
    pub generating: unsafe extern "C" fn() -> bool,
    pub callback: unsafe extern "C" fn(*mut c_void, u16, u32, u32) -> u16,
    pub callback_error: unsafe extern "C" fn(*const SpecHandle, u16, u16),
    pub grf_version: unsafe extern "C" fn(*const SpecHandle) -> u8,
    pub cargo_translation: unsafe extern "C" fn(*const SpecHandle, u8) -> u8,
    pub layout_size: unsafe extern "C" fn(*const LayoutHandle) -> usize,
    pub layout_tile: unsafe extern "C" fn(*const LayoutHandle, usize) -> LayoutTile,
    pub water_class: unsafe extern "C" fn(u32) -> u8,
    pub clear_tile: unsafe extern "C" fn(u32, u16),
    pub make_industry: unsafe extern "C" fn(u32, u16, u16, u32, u8),
    pub set_construction_counter: unsafe extern "C" fn(u32, u8),
    pub set_construction_stage: unsafe extern "C" fn(u32, u8),
    pub translated_gfx: unsafe extern "C" fn(u16) -> u16,
    pub animation_enabled: unsafe extern "C" fn(u16) -> bool,
    pub add_animated: unsafe extern "C" fn(u32, bool),
    pub build_window_dirty: unsafe extern "C" fn(),
    pub serve_neutral: unsafe extern "C" fn() -> bool,
    pub insert_near_station: unsafe extern "C" fn(*mut c_void, *mut StationHandle),
    pub clear_near_industries: unsafe extern "C" fn(*mut StationHandle),
    pub insert_neutral_industry: unsafe extern "C" fn(*mut StationHandle, *mut c_void),
    pub stations_around: unsafe extern "C" fn(
        *mut c_void,
        *mut c_void,
        unsafe extern "C" fn(*mut c_void, *mut StationHandle, u32) -> bool,
    ),
    pub add_delivery: unsafe extern "C" fn(*mut StationHandle, *mut c_void, u32),
}
const PROD_CHANGE_BUILD: u16 = 0x15f;
const DECIDE_COLOUR: u16 = 0x14a;
const INPUT_CARGO_TYPES: u16 = 0x14b;
const OUTPUT_CARGO_TYPES: u16 = 0x14c;
const ORIGINAL_CALLBACKS: u16 = (1 << 2) | (1 << 1) | (1 << 5) | (1 << 4) | (1 << 14);

fn add_location(owner: *mut Industry, tile: u32, size_x: u32) {
    let area = unsafe { super::lifecycle::location(owner) };
    let x = area.tile % size_x;
    let y = area.tile / size_x;
    let tx = tile % size_x;
    let ty = tile / size_x;
    let nx = x.min(tx);
    let ny = y.min(ty);
    unsafe {
        (*owner).metadata.tile = ny.wrapping_mul(size_x).wrapping_add(nx);
        (*owner).metadata.width = x
            .wrapping_add(u32::from(area.width))
            .max(tx.wrapping_add(1))
            .wrapping_sub(nx) as u16;
        (*owner).metadata.height = y
            .wrapping_add(u32::from(area.height))
            .max(ty.wrapping_add(1))
            .wrapping_sub(ny) as u16;
    }
}
struct Nearby<'a> {
    industry: *mut c_void,
    id: u16,
    lifecycle: &'a LifecycleServices,
    construction: &'a ConstructionServices,
}
unsafe extern "C" fn visit_station(
    context: *mut c_void,
    station: *mut StationHandle,
    tile: u32,
) -> bool {
    let context = unsafe { &*context.cast::<Nearby<'_>>() };
    if !unsafe { (context.lifecycle.belongs)(tile, context.id) } {
        return false;
    }
    unsafe {
        (context.construction.insert_near_station)(context.industry, station);
        (context.construction.add_delivery)(station, context.industry, tile);
    }
    false
}
fn populate(industry: *mut c_void, id: u16, l: &LifecycleServices, c: &ConstructionServices) {
    let neutral = unsafe { (l.neutral_station)(industry) };
    if !neutral.is_null() && !unsafe { (c.serve_neutral)() } {
        unsafe {
            (c.insert_near_station)(industry, neutral);
            (c.clear_near_industries)(neutral);
            (c.insert_neutral_industry)(neutral, industry);
        }
        return;
    }
    let mut context = Nearby {
        industry,
        id,
        lifecycle: l,
        construction: c,
    };
    unsafe {
        (c.stations_around)(industry, ptr::from_mut(&mut context).cast(), visit_station);
    }
}
fn cargo_callbacks(
    industry: *mut c_void,
    owner: *mut Industry,
    spec: *const SpecHandle,
    lists: &CargoLists,
    output: bool,
    g: &GenerationServices,
    c: &ConstructionServices,
) {
    let callback = if output {
        OUTPUT_CARGO_TYPES
    } else {
        INPUT_CARGO_TYPES
    };
    unsafe {
        if output {
            (*owner).produced.clear();
        } else {
            (*owner).accepted.clear();
        }
    }
    let unlimited = unsafe { (g.spec_behaviour)(spec) } & (1 << 18) != 0;
    let maxcargoes = if unlimited {
        16
    } else if output {
        2
    } else {
        3
    };
    for j in 0..maxcargoes {
        let res = unsafe { (c.callback)(industry, callback, j as u32, 0) };
        if res == CALLBACK_FAILED || res as u8 == u8::MAX {
            break;
        }
        if unsafe { (c.grf_version)(spec) } >= 8 && res >= 0x100 {
            unsafe { (c.callback_error)(spec, callback, res) };
            break;
        }
        let cargo = unsafe { (c.cargo_translation)(spec, res as u8) };
        if !valid(cargo) && !unlimited {
            unsafe {
                if output {
                    (*owner).produced.push(Produced {
                        cargo: u8::MAX,
                        ..Produced::default()
                    });
                } else {
                    (*owner).accepted.push(Accepted {
                        cargo: u8::MAX,
                        ..Accepted::default()
                    });
                }
            }
            continue;
        }
        let in_spec = if output {
            lists.produced.contains(&cargo)
        } else {
            lists.accepted.contains(&cargo)
        };
        if !in_spec {
            unsafe { (c.callback_error)(spec, callback, res) };
            break;
        }
        let duplicate = unsafe {
            if output {
                (0..j).any(|n| (*produced(owner, n)).cargo == cargo)
            } else {
                (0..j).any(|n| (*accepted(owner, n)).cargo == cargo)
            }
        };
        if duplicate {
            unsafe { (c.callback_error)(spec, callback, res) };
            break;
        }
        unsafe {
            if output {
                (*owner).produced.push(Produced {
                    cargo,
                    ..Produced::default()
                });
            } else {
                (*owner).accepted.push(Accepted {
                    cargo,
                    ..Accepted::default()
                });
            }
        }
    }
}
/// # Safety
/// Live newly allocated shell/owner, builder and immutable request/spec/layout.
/// Native town/station relationships remain native. Resolver and world callbacks
/// may reenter; raw scalar/slot access ends before every such call. Panics abort.
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_initialize(
    industry: *mut c_void,
    owner: *mut Industry,
    builder: *mut Builder,
    request: *const ConstructionRequest,
    s: *const Services,
    g: *const GenerationServices,
    l: *const LifecycleServices,
    c: *const ConstructionServices,
) {
    let r = unsafe { *request };
    let s = unsafe { &*s };
    let g = unsafe { &*g };
    let l = unsafe { &*l };
    let c = unsafe { &*c };
    let spec = unsafe { (g.spec)(r.kind) };
    unsafe {
        (*owner).metadata.tile = r.tile;
        (*owner).metadata.width = 1;
        (*owner).metadata.height = 1;
        (*owner).metadata.kind = r.kind;
        super::openttd_rust_industry_registry_insert(builder, r.kind, r.id);
    }
    let lists = unsafe { (c.cargo_lists)(spec) };
    let mut count = 0;
    for (n, cargo) in lists.produced.iter().enumerate() {
        if valid(*cargo) {
            count = n + 1;
        }
    }
    for n in 0..count {
        unsafe {
            (*owner).produced.push(Produced {
                cargo: lists.produced[n],
                ..Produced::default()
            });
        }
        let rate = unsafe { (s.production_rate)(industry, n as u32) };
        unsafe {
            (*produced(owner, n)).rate = rate;
        }
    }
    count = 0;
    for (n, cargo) in lists.accepted.iter().enumerate() {
        if valid(*cargo) {
            count = n + 1;
        }
    }
    for n in 0..count {
        unsafe {
            (*owner).accepted.push(Accepted {
                cargo: lists.accepted[n],
                ..Accepted::default()
            });
        }
    }
    let callbacks = unsafe { (c.callback_mask)(spec) };
    if unsafe { (s.economy_type)() } != 0 && callbacks & ORIGINAL_CALLBACKS == 0 {
        for n in 0..produced_count(owner) {
            let random = s.shared().random_range(256);
            let p = produced(owner, n);
            let rate = unsafe { u32::from((*p).rate) };
            unsafe {
                (*p).rate = ((random.wrapping_add(128)).wrapping_mul(rate) >> 8).min(255) as u8;
            }
        }
    }
    unsafe {
        (c.set_town)(industry, r.town);
        (*owner).metadata.owner = 16;
    }
    let random = s.shared().random() as u16;
    let f = fields(owner);
    unsafe {
        (*owner).metadata.random_colour = (random & 15) as u8;
        (*f).counter = random >> 4;
        (*owner).metadata.random = r.random_bits;
        (*f).was_cargo_delivered = 0;
    }
    let year = unsafe { (s.economy_year)() };
    unsafe {
        (*f).last_prod_year = year;
        (*owner).metadata.founder = r.founder;
        (*f).ctlflags = 0;
    }
    let date = unsafe { (c.calendar_date)() };
    unsafe {
        (*owner).metadata.construction_date = date;
    }
    let construction_type = if (g.environment.editor)() {
        3
    } else if unsafe { (c.generating)() } {
        2
    } else {
        1
    };
    unsafe {
        (*owner).metadata.construction_type = construction_type;
        (*owner).metadata.selected_layout = r.layout_index.wrapping_add(1) as u8;
        (*owner).metadata.exclusive_supplier = 255;
        (*owner).metadata.exclusive_consumer = 255;
        (*f).prod_level = 16;
    }
    if callbacks & (1 << 14) != 0 {
        let random = s.shared().random();
        let res = unsafe { (c.callback)(industry, PROD_CHANGE_BUILD, 0, random) };
        if res != CALLBACK_FAILED {
            if !(4..=128).contains(&res) {
                unsafe { (c.callback_error)(spec, PROD_CHANGE_BUILD, res) };
            } else {
                unsafe {
                    (*f).prod_level = res as u8;
                }
                super::recompute(industry, owner, s);
            }
        }
    }
    if unsafe { (c.generating)() } {
        if callbacks & (1 << 2) != 0 {
            unsafe {
                (s.production_callback)(industry, 1);
            }
            for n in 0..produced_count(owner) {
                let p = produced(owner, n);
                if valid(unsafe { (*p).cargo }) {
                    let waiting = unsafe { u32::from((*p).waiting) };
                    let production = unsafe { (s.scale_cargo)(waiting.wrapping_mul(8)) };
                    unsafe {
                        (*produced(owner, n)).history[usize::from(LAST_MONTH)].first =
                            production as u16;
                    }
                }
                unsafe {
                    (*produced(owner, n)).waiting = 0;
                }
            }
        }
        for n in 0..produced_count(owner) {
            let p = produced(owner, n);
            if valid(unsafe { (*p).cargo }) {
                let rate = unsafe { u32::from((*p).rate) };
                let production = unsafe { (s.scale_cargo)(rate.wrapping_mul(8)) };
                let p = produced(owner, n);
                unsafe {
                    (*p).history[usize::from(LAST_MONTH)].first = (*p).history
                        [usize::from(LAST_MONTH)]
                    .first
                    .wrapping_add(production as u16);
                }
            }
        }
        let month = unsafe { (s.economy_month)() };
        unsafe {
            super::update_history(&mut (*f).valid_history, u32::from(month));
        }
    }
    if callbacks & (1 << 11) != 0 {
        let res = unsafe { (c.callback)(industry, DECIDE_COLOUR, 0, 0) };
        if res != CALLBACK_FAILED {
            if res & 0x7ff0 != 0 {
                unsafe {
                    (c.callback_error)(spec, DECIDE_COLOUR, res);
                }
            }
            unsafe {
                (*owner).metadata.random_colour = (res & 15) as u8;
            }
        }
    }
    if callbacks & (1 << 12) != 0 {
        cargo_callbacks(industry, owner, spec, &lists, false, g, c);
    }
    if callbacks & (1 << 13) != 0 {
        cargo_callbacks(industry, owner, spec, &lists, true, g, c);
    }
    let count = unsafe { (c.layout_size)(r.layout) };
    for n in 0..count {
        let layout = unsafe { (c.layout_tile)(r.layout, n) };
        let tile = r.tile.wrapping_add(
            (i32::from(layout.y)
                .wrapping_mul(r.size_x as i32)
                .wrapping_add(i32::from(layout.x))) as u32,
        );
        if layout.gfx != 255 {
            add_location(owner, tile, r.size_x);
            let water = unsafe { (c.water_class)(tile) };
            unsafe {
                (c.clear_tile)(tile, (1 << 0) | (1 << 4) | (1 << 9));
            }
            let random = s.shared().random();
            unsafe {
                (c.make_industry)(tile, r.id, layout.gfx, random, water);
            }
            if unsafe { (c.generating)() } {
                unsafe {
                    (c.set_construction_counter)(tile, 3);
                    (c.set_construction_stage)(tile, 2);
                }
            }
            let gfx = unsafe { (c.translated_gfx)(layout.gfx) };
            if unsafe { (c.animation_enabled)(gfx) } {
                unsafe {
                    (c.add_animated)(tile, true);
                }
            }
        }
    }
    super::lifecycle::tiles(
        unsafe { super::lifecycle::location(owner) },
        r.size_x,
        |tile| unsafe {
            if (l.belongs)(tile, r.id) {
                super::openttd_rust_industry_animate_tile(tile, 0, 0x100, s, l);
            }
        },
    );
    let kind = unsafe { (*owner).metadata.kind };
    let behaviour = unsafe { (g.spec_behaviour)((g.spec)(kind)) };
    if behaviour & (1 << 6) != 0 {
        let map = Map {
            size_x: r.size_x,
            size_y: r.size_y,
            landscape: 0,
        };
        for _ in 0..50 {
            // Only the shared service table is borrowed across map/RNG calls.
            // Initialization observes landscape at the original plant branches.
            super::farm_with_landscape(industry, r.id, map, s, &|| unsafe { (s.landscape)() });
        }
    }
    unsafe {
        (s.directory_dirty)(0);
        (c.build_window_dirty)();
    }
    if !unsafe { (c.generating)() } {
        populate(industry, r.id, l, c);
    }
}
