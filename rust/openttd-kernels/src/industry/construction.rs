/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry initialization, tile planting and nearby-station policy. World callbacks can
//! inspect partially initialized owners.
use super::{
    Accepted, Builder, GenerationServices, Industry, LifecycleServices, Map, Produced, Services,
    SpecHandle, accepted, fields, produced, produced_count, valid,
};
use crate::command_type::DoCommandFlag;
use crate::company_type::{CompanyID, Owner};
use crate::history_type::LAST_MONTH;
use crate::industrytype::{self, IndustryBehaviour, IndustryBehaviours, IndustryConstructionType};
use crate::newgrf_animation_type::AnimationStatus;
use crate::newgrf_callbacks::CALLBACK_FAILED;
use crate::newgrf_callbacks::{CallbackID, IndustryCallbackMask, IndustryCallbackMasks};
use std::ffi::c_void;

/// One `IndustrySpec::layouts` entry, read in place.
pub use crate::industrytype::Layout as LayoutHandle;
pub use crate::town_type::TownHandle;
/// Tile `n` of a layout as an offset and gfx.
pub(crate) fn layout_tile(layout: *const LayoutHandle, n: usize) -> LayoutTile {
    let tile = unsafe { (*layout).as_slice()[n] };
    LayoutTile {
        x: tile.ti.x,
        y: tile.ti.y,
        gfx: tile.gfx,
    }
}
pub use crate::station_type::Station as StationHandle;
#[repr(C)]
pub struct CargoLists {
    pub produced: [u8; crate::industry_type::INDUSTRY_NUM_OUTPUTS],
    pub accepted: [u8; crate::industry_type::INDUSTRY_NUM_INPUTS],
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
    pub set_town: unsafe extern "C" fn(*mut c_void, *const TownHandle),
    pub generating: unsafe extern "C" fn() -> bool,
    pub callback: unsafe extern "C" fn(*mut c_void, u16, u32, u32) -> u16,
    pub callback_error: unsafe extern "C" fn(*const SpecHandle, u16, u16),
    pub water_class: unsafe extern "C" fn(u32) -> u8,
    pub clear_tile: unsafe extern "C" fn(u32, u16),
    pub make_industry: unsafe extern "C" fn(u32, u16, u16, u32, u8),
    pub set_construction_counter: unsafe extern "C" fn(u32, u8),
    pub set_construction_stage: unsafe extern "C" fn(u32, u8),
    pub translated_gfx: unsafe extern "C" fn(u16) -> u16,
    pub build_window_dirty: unsafe extern "C" fn(),
    pub serve_neutral: unsafe extern "C" fn() -> bool,
}
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
            .wrapping_add(u32::from(area.w))
            .max(tx.wrapping_add(1))
            .wrapping_sub(nx) as u16;
        (*owner).metadata.height = y
            .wrapping_add(u32::from(area.h))
            .max(ty.wrapping_add(1))
            .wrapping_sub(ny) as u16;
    }
}
/// `PopulateStationsNearby`.
fn populate(
    industry: *mut c_void,
    id: u16,
    owner: *mut Industry,
    s: &Services,
    l: &LifecycleServices,
    c: &ConstructionServices,
) {
    let cc = crate::station::Ctx::from_shared(s.shared());
    let shell = industry.cast::<crate::industry::tile::ShellHandle>();
    let neutral = (cc.c.industry_neutral)(shell);
    if !neutral.is_null() && !unsafe { (c.serve_neutral)() } {
        // The neutral station is used and other nearby stations are ignored.
        // SAFETY: Live owners; immediate mutations.
        unsafe {
            (*super::stations_near(owner)).insert(neutral, (cc.c.id)(neutral));
            crate::station::set_neutral_industry(
                (cc.c.owner)(neutral),
                shell,
                crate::industry_type::IndustryID(id),
            );
        }
        return;
    }
    crate::station::for_all_stations_around_tiles(super::location(owner), &cc, |st, tile| {
        if !unsafe { (l.belongs)(tile, id) } {
            return false;
        }
        // SAFETY: Live owner; immediate mutation.
        unsafe { (*super::stations_near(owner)).insert(st.station, st.id) };
        crate::station::add_industry_to_deliver(
            st.station,
            (cc.c.owner)(st.station),
            shell,
            tile,
            &cc,
        );
        false
    });
}
#[allow(clippy::too_many_arguments)]
fn cargo_callbacks(
    industry: *mut c_void,
    owner: *mut Industry,
    spec: *const SpecHandle,
    lists: &CargoLists,
    output: bool,
    c: &ConstructionServices,
    shared: &crate::services::Services,
) {
    let callback = if output {
        CallbackID::CBID_INDUSTRY_OUTPUT_CARGO_TYPES.0
    } else {
        CallbackID::CBID_INDUSTRY_INPUT_CARGO_TYPES.0
    };
    unsafe {
        if output {
            (*owner).produced.clear();
        } else {
            (*owner).accepted.clear();
        }
    }
    let unlimited = IndustryBehaviours(unsafe { (*spec).behaviour })
        .test(IndustryBehaviour::CargoTypesUnlimited);
    let maxcargoes = if unlimited {
        if output {
            crate::industry_type::INDUSTRY_NUM_OUTPUTS
        } else {
            crate::industry_type::INDUSTRY_NUM_INPUTS
        }
    } else if output {
        crate::industry_type::INDUSTRY_ORIGINAL_NUM_OUTPUTS
    } else {
        crate::industry_type::INDUSTRY_ORIGINAL_NUM_INPUTS
    };
    for j in 0..maxcargoes {
        let res = unsafe { (c.callback)(industry, callback, j as u32, 0) };
        if res == CALLBACK_FAILED || res as u8 == u8::MAX {
            break;
        }
        if (shared.grf_file_version)(unsafe { (*spec).grf_prop.grffile }) >= 8 && res >= 0x100 {
            unsafe { (c.callback_error)(spec, callback, res) };
            break;
        }
        let cargo = crate::cargo_registry::openttd_rust_cargo_translate(
            res as u8,
            unsafe { (*spec).grf_prop.grffile },
            false,
        );
        // Older GRFs rely on fixed cargo slot order and blank slots by returning
        // undefined cargo indices. Keep an invalid slot without ending the callbacks.
        if !valid(cargo) && !unlimited {
            unsafe {
                if output {
                    (*owner).produced.push(Produced {
                        cargo: crate::cargo_type::INVALID_CARGO,
                        ..Produced::default()
                    });
                } else {
                    (*owner).accepted.push(Accepted {
                        cargo: crate::cargo_type::INVALID_CARGO,
                        ..Accepted::default()
                    });
                }
            }
            continue;
        }
        // The cargo must occur in the specification; otherwise this is a GRF error.
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
        // Duplicate cargoes are also a GRF error.
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
    let spec = industrytype::industry(u16::from(r.kind));
    unsafe {
        (*owner).metadata.tile = r.tile;
        (*owner).metadata.width = 1;
        (*owner).metadata.height = 1;
        (*owner).metadata.kind = r.kind;
        super::openttd_rust_industry_registry_insert(builder, r.kind, r.id);
    }
    let lists = unsafe {
        CargoLists {
            produced: (*spec).produced_cargo,
            accepted: (*spec).accepts_cargo,
        }
    };
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
        let rate = unsafe { (*spec).production_rate[n] };
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
    let callbacks = unsafe { (*spec).callback_mask };
    if !super::original_economy(callbacks, unsafe { (s.economy_type)() }) {
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
        (*owner).metadata.owner = Owner::NONE.0;
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
    let date = crate::timer_game_common::calendar_date().0;
    unsafe {
        (*owner).metadata.construction_date = date;
    }
    let construction_type = if (g.environment.editor)() {
        IndustryConstructionType::ICT_SCENARIO_EDITOR
    } else if unsafe { (c.generating)() } {
        IndustryConstructionType::ICT_MAP_GENERATION
    } else {
        IndustryConstructionType::ICT_NORMAL_GAMEPLAY
    };
    unsafe {
        (*owner).metadata.construction_type = construction_type.0;
        // Zero means unknown (old save); known layouts are saved one-based.
        (*owner).metadata.selected_layout = r.layout_index.wrapping_add(1) as u8;
        (*owner).metadata.exclusive_supplier = CompanyID::INVALID.0;
        (*owner).metadata.exclusive_consumer = CompanyID::INVALID.0;
        (*f).prod_level = super::PRODLEVEL_DEFAULT;
    }
    // Call callbacks after the regular fields are initialized.
    if IndustryCallbackMasks(callbacks).test(IndustryCallbackMask::ProdChangeBuild) {
        let random = s.shared().random();
        let res = unsafe {
            (c.callback)(
                industry,
                CallbackID::CBID_INDUSTRY_PROD_CHANGE_BUILD.0,
                0,
                random,
            )
        };
        if res != CALLBACK_FAILED {
            if !(u16::from(super::PRODLEVEL_MINIMUM)..=u16::from(super::PRODLEVEL_MAXIMUM))
                .contains(&res)
            {
                unsafe {
                    (c.callback_error)(spec, CallbackID::CBID_INDUSTRY_PROD_CHANGE_BUILD.0, res);
                };
            } else {
                unsafe {
                    (*f).prod_level = res as u8;
                }
                super::recompute(owner);
            }
        }
    }
    if unsafe { (c.generating)() } {
        if IndustryCallbackMasks(callbacks).test(IndustryCallbackMask::Production256Ticks) {
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
    if IndustryCallbackMasks(callbacks).test(IndustryCallbackMask::DecideColour) {
        let res =
            unsafe { (c.callback)(industry, CallbackID::CBID_INDUSTRY_DECIDE_COLOUR.0, 0, 0) };
        if res != CALLBACK_FAILED {
            if res & 0x7ff0 != 0 {
                unsafe {
                    (c.callback_error)(spec, CallbackID::CBID_INDUSTRY_DECIDE_COLOUR.0, res);
                }
            }
            unsafe {
                (*owner).metadata.random_colour = (res & 15) as u8;
            }
        }
    }
    if IndustryCallbackMasks(callbacks).test(IndustryCallbackMask::InputCargoTypes) {
        cargo_callbacks(industry, owner, spec, &lists, false, c, s.shared());
    }
    if IndustryCallbackMasks(callbacks).test(IndustryCallbackMask::OutputCargoTypes) {
        cargo_callbacks(industry, owner, spec, &lists, true, c, s.shared());
    }
    let count = unsafe { (*r.layout).len() };
    for n in 0..count {
        let layout = layout_tile(r.layout, n);
        let tile = r.tile.wrapping_add(
            (i32::from(layout.y)
                .wrapping_mul(r.size_x as i32)
                .wrapping_add(i32::from(layout.x))) as u32,
        );
        if layout.gfx != crate::industry_map::GFX_WATERTILE_SPECIALCHECK {
            add_location(owner, tile, r.size_x);
            let water = unsafe { (c.water_class)(tile) };
            unsafe {
                (c.clear_tile)(
                    tile,
                    DoCommandFlag::EXECUTE
                        | DoCommandFlag::NO_TEST_TOWN_RATING
                        | DoCommandFlag::NO_MODIFY_TOWN_RATING,
                );
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
            if unsafe {
                (*industrytype::tile(gfx)).animation.status != AnimationStatus::NoAnimation.0
            } {
                crate::animated_tile::openttd_rust_animation_add(tile, true);
            }
        }
    }
    super::lifecycle::tiles(
        unsafe { super::lifecycle::location(owner) },
        r.size_x,
        |tile| unsafe {
            if (l.belongs)(tile.0, r.id) {
                super::openttd_rust_industry_animate_tile_abi(tile.0, 0, 0x100, s, l);
            }
        },
    );
    let kind = unsafe { (*owner).metadata.kind };
    let behaviour = unsafe { (*industrytype::industry(u16::from(kind))).behaviour };
    if IndustryBehaviours(behaviour).test(IndustryBehaviour::PlantOnBuild) {
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
        populate(industry, r.id, owner, s, l, c);
    }
}
