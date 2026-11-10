/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry initialization, tile planting and nearby-station policy. World callbacks can
//! inspect partially initialized owners.
use super::{
    Accepted, GenerationServices, Industry, LifecycleServices, Produced, Services, SpecHandle,
    TransactionContext, accepted, fields, produced,
};
use crate::bitmath_func::gb;
use crate::cargo_type::{CargoType, is_valid_cargo_type};
use crate::command_type::DoCommandFlag;
use crate::company_type::Owner;
use crate::economy_type::EconomyType;
use crate::history::update_valid_history_year;
use crate::history_type::LAST_MONTH;
use crate::industry_type::{
    INDUSTRY_NUM_INPUTS, INDUSTRY_NUM_OUTPUTS, INDUSTRY_ORIGINAL_NUM_INPUTS,
    INDUSTRY_ORIGINAL_NUM_OUTPUTS, IndustryAnimationTrigger, IndustryGfx, IndustryID, IndustryType,
};
use crate::industrytype::{
    IndustryBehaviour, IndustryBehaviours, IndustryConstructionType, get_industry_spec,
    get_industry_tile_spec,
};
use crate::newgrf_animation_type::AnimationStatus;
use crate::newgrf_callbacks::CALLBACK_FAILED;
use crate::newgrf_callbacks::{CallbackID, IndustryCallbackMask, IndustryCallbackMasks};
use crate::tile_type::TileIndex;
use crate::tilearea::TileArea;
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
    pub produced: [u8; INDUSTRY_NUM_OUTPUTS],
    pub accepted: [u8; INDUSTRY_NUM_INPUTS],
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
/// `PopulateStationsNearby`: populate an industry's list of nearby stations, and the
/// stations' lists of nearby industries.
fn populate(
    industry: *mut c_void,
    id: IndustryID,
    owner: *mut Industry,
    s: &Services,
    l: &LifecycleServices,
    c: &ConstructionServices,
) {
    let cc = crate::station::Ctx::from_shared(s.shared());
    let shell = industry.cast::<crate::industry::tile::ShellHandle>();
    let neutral = (cc.c.industry_neutral)(shell);
    if !neutral.is_null() && !unsafe { (c.serve_neutral)() } {
        // Industry has a neutral station. Use it and ignore any other nearby stations.
        // SAFETY: Live owners; immediate mutations.
        unsafe {
            (*super::stations_near(owner)).insert(neutral, (cc.c.id)(neutral));
            crate::station::set_neutral_industry((cc.c.owner)(neutral), shell, id);
        }
        return;
    }
    crate::station::for_all_stations_around_tiles(super::location(owner), &cc, |st, tile| {
        if !unsafe { (l.belongs)(tile.0, id.0) } {
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
/// The `CBID_INDUSTRY_INPUT_CARGO_TYPES` or `CBID_INDUSTRY_OUTPUT_CARGO_TYPES` branch of
/// `DoCreateNewIndustry`; `output` selects the produced list.
#[allow(
    clippy::cast_possible_truncation,
    reason = "DoCreateNewIndustry's cargo-type callbacks keep the original's implicit C++ integer conversions"
)]
fn cargo_callbacks(
    industry: *mut c_void,
    owner: *mut Industry,
    indspec: *const SpecHandle,
    lists: &CargoLists,
    output: bool,
    c: &ConstructionServices,
    shared: &crate::services::Services,
) {
    let callback = if output {
        CallbackID::CBID_INDUSTRY_OUTPUT_CARGO_TYPES
    } else {
        CallbackID::CBID_INDUSTRY_INPUT_CARGO_TYPES
    };
    // Clear all input or output cargo types.
    unsafe {
        if output {
            (*owner).produced.clear();
        } else {
            (*owner).accepted.clear();
        }
    }
    // Query actual types.
    let unlimited = IndustryBehaviours(unsafe { (*indspec).behaviour })
        .test(IndustryBehaviour::CargoTypesUnlimited);
    let maxcargoes = if unlimited {
        if output {
            INDUSTRY_NUM_OUTPUTS
        } else {
            INDUSTRY_NUM_INPUTS
        }
    } else if output {
        INDUSTRY_ORIGINAL_NUM_OUTPUTS
    } else {
        INDUSTRY_ORIGINAL_NUM_INPUTS
    };
    for j in 0..maxcargoes {
        let res = unsafe { (c.callback)(industry, callback.0, j as u32, 0) };
        if res == CALLBACK_FAILED || gb(u32::from(res), 0, 8) == u32::from(u8::MAX) {
            break;
        }
        if (shared.grf_file_version)(unsafe { (*indspec).grf_prop.grffile }) >= 8 && res >= 0x100 {
            unsafe { (c.callback_error)(indspec, callback.0, res) };
            break;
        }
        let cargo = CargoType(crate::cargo_registry::openttd_rust_cargo_translate(
            gb(u32::from(res), 0, 8) as u8,
            unsafe { (*indspec).grf_prop.grffile },
            false,
        ));
        // Industries without "unlimited" cargo types support depend on the specific
        // order/slots of cargo types. They need to be able to blank out specific slots
        // without aborting the callback sequence, and solve this by returning undefined
        // cargo indexes. Skip these.
        if !is_valid_cargo_type(cargo.0) && !unlimited {
            // As slots are allocated as needed now, this means we do need to add a slot
            // for the invalid cargo.
            unsafe {
                if output {
                    (*owner).produced.push(Produced {
                        cargo: CargoType::INVALID_CARGO.0,
                        ..Produced::default()
                    });
                } else {
                    (*owner).accepted.push(Accepted {
                        cargo: CargoType::INVALID_CARGO.0,
                        ..Accepted::default()
                    });
                }
            }
            continue;
        }
        // Verify valid cargo.
        let is_listed = if output {
            lists.produced.contains(&cargo.0)
        } else {
            lists.accepted.contains(&cargo.0)
        };
        if !is_listed {
            // Cargo not in spec, error in NewGRF.
            unsafe { (c.callback_error)(indspec, callback.0, res) };
            break;
        }
        let duplicate = unsafe {
            if output {
                (0..j).any(|n| (*produced(owner, n)).cargo == cargo.0)
            } else {
                (0..j).any(|n| (*accepted(owner, n)).cargo == cargo.0)
            }
        };
        if duplicate {
            // Duplicate cargo.
            unsafe { (c.callback_error)(indspec, callback.0, res) };
            break;
        }
        unsafe {
            if output {
                (*owner).produced.push(Produced {
                    cargo: cargo.0,
                    ..Produced::default()
                });
            } else {
                (*owner).accepted.push(Accepted {
                    cargo: cargo.0,
                    ..Accepted::default()
                });
            }
        }
    }
}
/// `DoCreateNewIndustry`: put an industry on the map. `request` carries the north tile,
/// type, layout and its index, nearest town, founder (`OWNER_NONE` in case of random
/// construction) and initial random bits; `context` carries the transaction's services.
///
/// # Safety
/// Live newly allocated shell/owner, builder and immutable request/spec/layout.
/// Native town/station relationships remain native. Resolver and world callbacks
/// may reenter; raw scalar/slot access ends before every such call. Panics abort.
#[allow(
    clippy::cast_possible_truncation,
    reason = "DoCreateNewIndustry keeps the original's implicit C++ integer conversions"
)]
#[allow(
    clippy::too_many_lines,
    reason = "DoCreateNewIndustry is ported as the original's single body; its length is tracked in the Clippy debt ledger"
)]
pub(crate) unsafe fn do_create_new_industry(
    industry: *mut c_void,
    owner: *mut Industry,
    request: &ConstructionRequest,
    context: &TransactionContext,
) {
    let req = *request;
    let prod = unsafe { &*context.production };
    let generation: &GenerationServices = unsafe { &*context.generation };
    let life = unsafe { &*context.lifecycle };
    let cons = unsafe { &*context.construction };
    let (tile, kind, id) = (
        TileIndex(req.tile),
        IndustryType(req.kind),
        IndustryID(req.id),
    );
    let map = crate::map_func::Map::from_log_x(req.size_x.ilog2());
    let indspec = get_industry_spec(kind);
    unsafe {
        (*owner).metadata.tile = tile.0;
        (*owner).metadata.width = 1;
        (*owner).metadata.height = 1;
        (*owner).metadata.kind = kind.0;
        super::openttd_rust_industry_registry_insert(context.builder, kind.0, id.0);
    }
    let lists = unsafe {
        CargoLists {
            produced: (*indspec).produced_cargo,
            accepted: (*indspec).accepts_cargo,
        }
    };
    let mut produced_count = 0;
    for (index, cargo) in lists.produced.iter().enumerate() {
        if is_valid_cargo_type(*cargo) {
            produced_count = index + 1;
        }
    }
    for index in 0..produced_count {
        unsafe {
            (*owner).produced.push(Produced {
                cargo: lists.produced[index],
                ..Produced::default()
            });
        }
        unsafe { (*produced(owner, index)).rate = (*indspec).production_rate[index] };
    }
    let mut accepted_count = 0;
    for (index, cargo) in lists.accepted.iter().enumerate() {
        if is_valid_cargo_type(*cargo) {
            accepted_count = index + 1;
        }
    }
    for index in 0..accepted_count {
        unsafe {
            (*owner).accepted.push(Accepted {
                cargo: lists.accepted[index],
                ..Accepted::default()
            });
        }
    }
    let callbacks = IndustryCallbackMasks(unsafe { (*indspec).callback_mask });
    // Randomize initial production if non-original economy is used and there are no
    // production related callbacks.
    if !super::uses_original_economy(callbacks, EconomyType(unsafe { (prod.economy_type)() })) {
        for slot in 0..super::produced_count(owner) {
            let random = prod.shared().random_range(256);
            let p = produced(owner, slot);
            let rate = unsafe { u32::from((*p).rate) };
            unsafe {
                (*p).rate = ((random.wrapping_add(128)).wrapping_mul(rate) >> 8)
                    .min(u32::from(u8::MAX)) as u8;
            }
        }
    }
    unsafe {
        (cons.set_town)(industry, req.town);
        (*owner).metadata.owner = Owner::NONE.0;
    }
    let random = prod.shared().random() as u16;
    let state = fields(owner);
    unsafe {
        (*owner).metadata.random_colour = gb(u32::from(random), 0, 4) as u8;
        (*state).counter = gb(u32::from(random), 4, 12) as u16;
        (*owner).metadata.random = req.random_bits;
        (*state).was_cargo_delivered = 0;
    }
    let year = unsafe { (prod.economy_year)() };
    unsafe {
        (*state).last_prod_year = year;
        (*owner).metadata.founder = req.founder;
        (*state).ctlflags = 0;
    }
    let date = crate::timer_game_common::calendar_date().0;
    unsafe { (*owner).metadata.construction_date = date };
    let construction_type = if (generation.environment.editor)() {
        IndustryConstructionType::ICT_SCENARIO_EDITOR
    } else if unsafe { (cons.generating)() } {
        IndustryConstructionType::ICT_MAP_GENERATION
    } else {
        IndustryConstructionType::ICT_NORMAL_GAMEPLAY
    };
    unsafe {
        (*owner).metadata.construction_type = construction_type.0;
        // Adding 1 here makes it conform to specs of var44 of varaction2 for industries:
        // 0 = created prior of newindustries, else chosen layout + 1.
        (*owner).metadata.selected_layout = req.layout_index.wrapping_add(1) as u8;
        (*owner).metadata.exclusive_supplier = Owner::INVALID.0;
        (*owner).metadata.exclusive_consumer = Owner::INVALID.0;
        (*state).prod_level = super::PRODLEVEL_DEFAULT;
    }
    // Call callbacks after the regular fields got initialised.
    if callbacks.test(IndustryCallbackMask::ProdChangeBuild) {
        let random = prod.shared().random();
        let res = unsafe {
            (cons.callback)(
                industry,
                CallbackID::CBID_INDUSTRY_PROD_CHANGE_BUILD.0,
                0,
                random,
            )
        };
        if res != CALLBACK_FAILED {
            if (u16::from(super::PRODLEVEL_MINIMUM)..=u16::from(super::PRODLEVEL_MAXIMUM))
                .contains(&res)
            {
                unsafe {
                    (*state).prod_level = res as u8;
                }
                super::recompute_production_multipliers(owner);
            } else {
                unsafe {
                    (cons.callback_error)(
                        indspec,
                        CallbackID::CBID_INDUSTRY_PROD_CHANGE_BUILD.0,
                        res,
                    );
                };
            }
        }
    }
    if unsafe { (cons.generating)() } {
        if callbacks.test(IndustryCallbackMask::Production256Ticks) {
            unsafe {
                (prod.production_callback)(industry, 1);
            }
            for slot in 0..super::produced_count(owner) {
                let p = produced(owner, slot);
                if is_valid_cargo_type(unsafe { (*p).cargo }) {
                    let waiting = unsafe { u32::from((*p).waiting) };
                    let production = unsafe { (prod.scale_cargo)(waiting.wrapping_mul(8)) };
                    unsafe {
                        (*produced(owner, slot)).history[usize::from(LAST_MONTH)].first =
                            production as u16;
                    }
                }
                unsafe {
                    (*produced(owner, slot)).waiting = 0;
                }
            }
        }
        for slot in 0..super::produced_count(owner) {
            let p = produced(owner, slot);
            if is_valid_cargo_type(unsafe { (*p).cargo }) {
                let rate = unsafe { u32::from((*p).rate) };
                let production = unsafe { (prod.scale_cargo)(rate.wrapping_mul(8)) };
                let p = produced(owner, slot);
                unsafe {
                    (*p).history[usize::from(LAST_MONTH)].first = (*p).history
                        [usize::from(LAST_MONTH)]
                    .first
                    .wrapping_add(production as u16);
                }
            }
        }
        let month = unsafe { (prod.economy_month)() };
        unsafe {
            (*state).valid_history =
                update_valid_history_year((*state).valid_history, u32::from(month));
        }
    }
    if callbacks.test(IndustryCallbackMask::DecideColour) {
        let res =
            unsafe { (cons.callback)(industry, CallbackID::CBID_INDUSTRY_DECIDE_COLOUR.0, 0, 0) };
        if res != CALLBACK_FAILED {
            if gb(u32::from(res), 4, 11) != 0 {
                unsafe {
                    (cons.callback_error)(indspec, CallbackID::CBID_INDUSTRY_DECIDE_COLOUR.0, res);
                }
            }
            unsafe { (*owner).metadata.random_colour = gb(u32::from(res), 0, 4) as u8 };
        }
    }
    if callbacks.test(IndustryCallbackMask::InputCargoTypes) {
        cargo_callbacks(industry, owner, indspec, &lists, false, cons, prod.shared());
    }
    if callbacks.test(IndustryCallbackMask::OutputCargoTypes) {
        cargo_callbacks(industry, owner, indspec, &lists, true, cons, prod.shared());
    }
    // Plant the tiles.
    let count = unsafe { (*req.layout).len() };
    for slot in 0..count {
        let it = layout_tile(req.layout, slot);
        let cur_tile = TileIndex(
            tile.0
                .wrapping_add(map.diff(i32::from(it.x), i32::from(it.y))),
        );
        if IndustryGfx(it.gfx) != IndustryGfx::GFX_WATERTILE_SPECIALCHECK {
            let mut location = super::location(owner);
            location.add(cur_tile.0, map);
            set_location(owner, location);
            let wc = unsafe { (cons.water_class)(cur_tile.0) };
            unsafe {
                (cons.clear_tile)(
                    cur_tile.0,
                    DoCommandFlag::EXECUTE
                        | DoCommandFlag::NO_TEST_TOWN_RATING
                        | DoCommandFlag::NO_MODIFY_TOWN_RATING,
                );
            }
            let random = prod.shared().random();
            unsafe {
                (cons.make_industry)(cur_tile.0, id.0, it.gfx, random, wc);
            }
            if unsafe { (cons.generating)() } {
                unsafe {
                    (cons.set_construction_counter)(cur_tile.0, 3);
                    (cons.set_construction_stage)(cur_tile.0, 2);
                }
            }
            // `it.gfx` is stored in the map. But the translated ID `cur_gfx` is the
            // interesting one.
            let cur_gfx = IndustryGfx(unsafe { (cons.translated_gfx)(it.gfx) });
            let its = get_industry_tile_spec(cur_gfx);
            if unsafe { (*its).animation.status != AnimationStatus::NoAnimation.0 } {
                crate::animated_tile::openttd_rust_animation_add(cur_tile.0, true);
            }
        }
    }
    // Call callbacks after all tiles have been created.
    super::lifecycle::tiles(super::location(owner), req.size_x, |cur_tile| unsafe {
        if (life.belongs)(cur_tile.0, id.0) {
            // There are no shared random bits, consistent with "MakeIndustryTileBigger"
            // in tile loop. So, trigger tiles individually; `first_call` adds 0x100.
            super::openttd_rust_industry_animate_tile_abi(
                cur_tile.0,
                IndustryAnimationTrigger::ConstructionStageChanged.0,
                0x100,
                prod,
                life,
            );
        }
    });
    let kind = IndustryType(unsafe { (*owner).metadata.kind });
    let behaviour = IndustryBehaviours(unsafe { (*get_industry_spec(kind)).behaviour });
    if behaviour.test(IndustryBehaviour::PlantOnBuild) {
        let map = super::Map {
            size_x: req.size_x,
            size_y: req.size_y,
            landscape: 0,
        };
        for _ in 0..50 {
            // Only the shared service table is borrowed across map/RNG calls.
            // Initialization observes landscape at the original plant branches.
            super::plant_random_farm_field(industry, id, map, prod, &|| prod.landscape_type());
        }
    }
    unsafe {
        (prod.directory_dirty)(super::IndustryDirectoryInvalidateWindowData::IDIWD_FORCE_REBUILD.0);
        (cons.build_window_dirty)();
    }
    if !unsafe { (cons.generating)() } {
        populate(industry, id, owner, prod, life, cons);
    }
}
/// Write `location` back to `Industry::location`.
fn set_location(owner: *mut Industry, location: TileArea) {
    // SAFETY: Live owner supplied by the caller's resolved shell; immediate writes.
    unsafe {
        (*owner).metadata.tile = location.tile;
        (*owner).metadata.width = location.w;
        (*owner).metadata.height = location.h;
    }
}
