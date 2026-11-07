/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry tile construction, builtin animation and tile-loop control. Map writes and
//! resolver callbacks can reenter.
// Retain the original tick masks for source comparison.
#![allow(clippy::verbose_bit_mask)]
use super::{
    ConstructionServices, GenerationServices, Industry, IndustryCost, IndustryCostServices,
    LifecycleServices, Services, SpecHandle,
};
use crate::cargo_type::INVALID_CARGO;
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::Owner;
use crate::economy_type::PR_BUILD_FOUNDATION;
use crate::industry_map::{
    GFX_BUBBLE_CATCHER, GFX_BUBBLE_GENERATOR, GFX_COAL_MINE_TOWER_ANIMATED,
    GFX_COAL_MINE_TOWER_NOT_ANIMATED, GFX_COPPER_MINE_CHIMNEY, GFX_COPPER_MINE_TOWER_ANIMATED,
    GFX_COPPER_MINE_TOWER_NOT_ANIMATED, GFX_GOLD_MINE_TOWER_ANIMATED,
    GFX_GOLD_MINE_TOWER_NOT_ANIMATED, GFX_OILRIG_1, GFX_OILWELL_ANIMATED_1, GFX_OILWELL_ANIMATED_3,
    GFX_OILWELL_NOT_ANIMATED, GFX_PLASTIC_FOUNTAIN_ANIMATED_1, GFX_PLASTIC_FOUNTAIN_ANIMATED_8,
    GFX_POWERPLANT_CHIMNEY, GFX_POWERPLANT_SPARKS, GFX_SUGAR_MINE_SIEVE, GFX_TOFFEE_QUARRY,
    GFX_TOY_FACTORY,
};
use crate::industry_type::{
    INDUSTRY_COMPLETED, INDUSTRY_NUM_INPUTS, INDUSTRY_ORIGINAL_NUM_INPUTS,
    IndustryAnimationTrigger, IndustryGfx, IndustryRandomTrigger,
};
use crate::industrytype::{
    self, IndustryBehaviour, IndustryBehaviours, IndustryTileSpecialFlag, IndustryTileSpecialFlags,
};
use crate::newgrf_animation_type::AnimationStatus;
use crate::newgrf_callbacks::CALLBACK_FAILED;
use crate::newgrf_callbacks::{CallbackID, IndustryTileCallbackMask, IndustryTileCallbackMasks};
use crate::slope_func::{get_slope_max_z, is_steep_slope};
use crate::sound_type::{SoundFx, SoundID};
use crate::tile_type::{MP_INDUSTRY, TILE_SIZE};
use std::{ffi::c_void, ptr};

/// The native `Industry` pool shell.
pub use crate::industry_type::Industry as ShellHandle;
#[repr(C)]
pub struct TileIndustry {
    pub shell: *mut ShellHandle,
    pub owner: *mut Industry,
}
pub use crate::industrytype::IndustryTileSpec as TileSpec;
#[repr(C)]
pub struct TileAcceptance {
    pub cargo: [u8; INDUSTRY_NUM_INPUTS],
    pub amount: [i8; INDUSTRY_NUM_INPUTS],
    pub accepts_all: bool,
    pub accepts_callback: bool,
    pub amount_callback: bool,
}
#[repr(C)]
pub struct TileServices {
    pub shared: *const crate::services::Services,
    pub gfx: unsafe extern "C" fn(u32) -> u16,
    pub set_gfx: unsafe extern "C" fn(u32, u16),
    pub frame: unsafe extern "C" fn(u32) -> u8,
    pub set_frame: unsafe extern "C" fn(u32, u8),
    pub animation_loop: unsafe extern "C" fn(u32) -> u8,
    pub set_animation_loop: unsafe extern "C" fn(u32, u8),

    pub construction_counter: unsafe extern "C" fn(u32) -> u8,
    pub construction_stage: unsafe extern "C" fn(u32) -> u8,
    pub completed: unsafe extern "C" fn(u32) -> bool,
    pub set_completed: unsafe extern "C" fn(u32),
    pub reset_construction: unsafe extern "C" fn(u32),
    pub industry: unsafe extern "C" fn(u32) -> TileIndustry,
    pub industry_index: unsafe extern "C" fn(u32) -> u16,
    pub toffee_offset: unsafe extern "C" fn(u8) -> u8,
    pub on_water: unsafe extern "C" fn(u32) -> bool,
    pub transport: unsafe extern "C" fn(u32) -> bool,
    pub build_oilrig: unsafe extern "C" fn(u32),
    pub pixel_max_z: unsafe extern "C" fn(u32) -> i32,
    pub callback: unsafe extern "C" fn(u32, u16, u16, u32, u32) -> u16,
}
fn chance(s: &Services, a: u32, b: u32) -> bool {
    crate::random_func::chance16_i(a, b, s.shared().random())
}
fn sound(tile: u32, sound: SoundID, s: &Services) {
    if unsafe { s.shared().settings().ambient.read() } {
        unsafe { (s.play_sound)(sound.0, tile) };
    }
}
fn dirty(tile: u32, s: &Services) {
    (s.shared().mark_dirty)(tile);
}
fn chimney(tile: u32, map: crate::services::Map, t: &TileServices) {
    let x = map.tile_x(tile).wrapping_mul(TILE_SIZE);
    let y = map.tile_y(tile).wrapping_mul(TILE_SIZE);
    let z = unsafe { (t.pixel_max_z)(tile) };
    crate::effect::openttd_rust_effect_create(
        x.wrapping_add(15) as i32,
        y.wrapping_add(14) as i32,
        z.wrapping_add(59),
        crate::effectvehicle_func::EV_CHIMNEY_SMOKE,
    );
}
fn bigger(
    tile: u32,
    map: crate::services::Map,
    s: &Services,
    c: &ConstructionServices,
    l: &LifecycleServices,
    t: &TileServices,
) {
    let counter = unsafe { (t.construction_counter)(tile) }.wrapping_add(1);
    if counter != 4 {
        unsafe { (c.set_construction_counter)(tile, counter) };
        return;
    }
    let stage = unsafe { (t.construction_stage)(tile) }.wrapping_add(1);
    unsafe {
        (c.set_construction_counter)(tile, 0);
        (c.set_construction_stage)(tile, stage);
        super::openttd_rust_industry_animate_tile_abi(
            tile,
            IndustryAnimationTrigger::ConstructionStageChanged.0,
            0,
            s,
            l,
        );
        if stage == INDUSTRY_COMPLETED as u8 {
            (t.set_completed)(tile);
        }
    }
    dirty(tile, s);
    if !unsafe { (t.completed)(tile) } {
        return;
    }
    let gfx = unsafe { (t.gfx)(tile) };
    if gfx >= crate::industry_type::NEW_INDUSTRYTILEOFFSET {
        return;
    }
    match gfx {
        GFX_POWERPLANT_CHIMNEY => chimney(tile, map, t),
        GFX_OILRIG_1 => {
            // Do not require an industry tile after the first two GFX_OILRIG_1
            // tiles. Build the station after checking both belong to this industry.
            let other = tile.wrapping_add(map.size_x());
            if (s.shared().tile_type)(other) == MP_INDUSTRY
                && unsafe { (t.gfx)(other) } == GFX_OILRIG_1
            {
                let own = unsafe { (t.industry_index)(tile) };
                let other = unsafe { (t.industry_index)(other) };
                if own == other {
                    unsafe { (t.build_oilrig)(tile) };
                }
            }
        }
        GFX_TOY_FACTORY | GFX_BUBBLE_CATCHER | GFX_TOFFEE_QUARRY => unsafe {
            (t.set_frame)(tile, 0);
            (t.set_animation_loop)(tile, 0);
        },
        GFX_PLASTIC_FOUNTAIN_ANIMATED_1..=GFX_PLASTIC_FOUNTAIN_ANIMATED_8 => {
            crate::animated_tile::openttd_rust_animation_add(tile, false)
        }
        _ => {}
    }
}
fn mine(tile: u32, s: &Services, t: &TileServices) {
    let state = ((s.shared().tick_counter)() & 0x7ff) as i32 - 0x400;
    if state < 0 {
        return;
    }
    if state < 0x1a0 {
        if !(0x20..0x180).contains(&state) {
            let frame = unsafe { (t.frame)(tile) };
            if frame & 0x40 == 0 {
                unsafe { (t.set_frame)(tile, frame | 0x40) };
                sound(tile, SoundFx::SND_0B_MINE.into(), s);
            }
            if state & 7 != 0 {
                return;
            }
        } else if state & 3 != 0 {
            return;
        }
        let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1) | 0x40;
        if frame > 0xc2 {
            frame = 0xc0;
        }
        unsafe { (t.set_frame)(tile, frame) };
        dirty(tile, s);
    } else if (0x200..0x3a0).contains(&state) {
        let mask = if !(0x220..0x380).contains(&state) {
            7
        } else {
            3
        };
        if state & mask != 0 {
            return;
        }
        let mut frame = (unsafe { (t.frame)(tile) } & 0xbf).wrapping_sub(1);
        if frame < 0x80 {
            frame = 0x82;
        }
        unsafe { (t.set_frame)(tile, frame) };
        dirty(tile, s);
    }
}
/// # Safety
/// Industry tile; immutable service tables outlive the synchronous invocation.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_animate(
    tile: u32,
    s: *const Services,
    c: *const ConstructionServices,
    t: *const TileServices,
) {
    let (s, c, t) = unsafe { (&*s, &*c, &*t) };
    let gfx = unsafe { (t.gfx)(tile) };
    if unsafe { (*industrytype::tile(gfx)).animation.status != AnimationStatus::NoAnimation.0 } {
        crate::animated_tile::openttd_rust_animate_industry(tile);
        return;
    }
    match gfx {
        GFX_SUGAR_MINE_SIEVE if (s.shared().tick_counter)() & 1 == 0 => {
            let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1);
            if unsafe { s.shared().settings().ambient.read() } {
                match frame & 7 {
                    2 => unsafe { (s.play_sound)(u16::from(SoundFx::SND_2D_SUGAR_MINE_1.0), tile) },
                    6 => unsafe { (s.play_sound)(u16::from(SoundFx::SND_29_SUGAR_MINE_2.0), tile) },
                    _ => {}
                }
            }
            if frame >= 96 {
                frame = 0;
                crate::animated_tile::openttd_rust_animation_delete(tile, false);
            }
            unsafe { (t.set_frame)(tile, frame) };
            dirty(tile, s);
        }
        GFX_TOFFEE_QUARRY if (s.shared().tick_counter)() & 3 == 0 => {
            let mut frame = unsafe { (t.frame)(tile) };
            if unsafe { (t.toffee_offset)(frame) } == 0xff {
                sound(tile, SoundFx::SND_30_TOFFEE_QUARRY.into(), s);
            }
            frame = frame.wrapping_add(1);
            if frame >= 70 {
                frame = 0;
                crate::animated_tile::openttd_rust_animation_delete(tile, false);
            }
            unsafe { (t.set_frame)(tile, frame) };
            dirty(tile, s);
        }
        GFX_BUBBLE_CATCHER if (s.shared().tick_counter)() & 1 == 0 => {
            let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1);
            if frame >= 40 {
                frame = 0;
                crate::animated_tile::openttd_rust_animation_delete(tile, false);
            }
            unsafe { (t.set_frame)(tile, frame) };
            dirty(tile, s);
        }
        GFX_POWERPLANT_SPARKS if (s.shared().tick_counter)() & 3 == 0 => {
            let frame = unsafe { (t.frame)(tile) };
            if frame == 6 {
                unsafe {
                    (t.set_frame)(tile, 0);
                    crate::animated_tile::openttd_rust_animation_delete(tile, false);
                }
            } else {
                unsafe { (t.set_frame)(tile, frame.wrapping_add(1)) };
            }
            dirty(tile, s);
        }
        GFX_TOY_FACTORY if (s.shared().tick_counter)() & 1 == 0 => {
            let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1);
            match frame {
                1 => sound(tile, SoundFx::SND_2C_TOY_FACTORY_1.into(), s),
                23 => sound(tile, SoundFx::SND_2B_TOY_FACTORY_2.into(), s),
                28 => sound(tile, SoundFx::SND_2A_TOY_FACTORY_3.into(), s),
                _ => {
                    if frame >= 50 {
                        let mut count =
                            u32::from(unsafe { (t.animation_loop)(tile) }).wrapping_add(1);
                        frame = 0;
                        if count >= 8 {
                            count = 0;
                            crate::animated_tile::openttd_rust_animation_delete(tile, false);
                        }
                        unsafe { (t.set_animation_loop)(tile, count as u8) };
                    }
                }
            }
            unsafe { (t.set_frame)(tile, frame) };
            dirty(tile, s);
        }
        GFX_PLASTIC_FOUNTAIN_ANIMATED_1..=GFX_PLASTIC_FOUNTAIN_ANIMATED_8
            if (s.shared().tick_counter)() & 3 == 0 =>
        {
            unsafe {
                (t.set_gfx)(
                    tile,
                    if gfx < GFX_PLASTIC_FOUNTAIN_ANIMATED_8 {
                        gfx.wrapping_add(1)
                    } else {
                        GFX_PLASTIC_FOUNTAIN_ANIMATED_1
                    },
                )
            };
            dirty(tile, s);
        }
        GFX_OILWELL_ANIMATED_1..=32 if (s.shared().tick_counter)() & 7 == 0 => {
            let stop = chance(s, 1, 7);
            let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1);
            let mut gfx = gfx;
            let last = if frame == 4 {
                frame = 0;
                gfx = gfx.wrapping_add(1);
                if gfx == GFX_OILWELL_ANIMATED_3 + 1 {
                    gfx = GFX_OILWELL_ANIMATED_1;
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if last && stop {
                unsafe {
                    (t.set_gfx)(tile, GFX_OILWELL_NOT_ANIMATED);
                    (c.set_construction_stage)(tile, INDUSTRY_COMPLETED as u8);
                    crate::animated_tile::openttd_rust_animation_delete(tile, false);
                }
            } else {
                unsafe {
                    (t.set_frame)(tile, frame);
                    (t.set_gfx)(tile, gfx);
                }
            }
            dirty(tile, s);
        }
        GFX_COAL_MINE_TOWER_ANIMATED
        | GFX_COPPER_MINE_TOWER_ANIMATED
        | GFX_GOLD_MINE_TOWER_ANIMATED => mine(tile, s, t),
        _ => {}
    }
}
fn bubble(tile: u32, s: &Services, map: crate::services::Map) {
    sound(tile, SoundFx::SND_2E_BUBBLE_GENERATOR.into(), s);
    let dir = (s.shared().random() & 3) as usize;
    let x = map
        .tile_x(tile)
        .wrapping_mul(TILE_SIZE)
        .wrapping_add([11_i32, 0, -4, -14][dir] as u32) as i32;
    let y = map
        .tile_y(tile)
        .wrapping_mul(TILE_SIZE)
        .wrapping_add([-4_i32, -10, -4, 1][dir] as u32) as i32;
    let vehicle = crate::effect::openttd_rust_effect_create_above(
        x,
        y,
        [49, 59, 60, 65][dir],
        crate::effectvehicle_func::EV_BUBBLE,
    );
    if !vehicle.is_null() {
        crate::effect::set_substate(vehicle, dir as u8);
    }
}
/// # Safety
/// Live map tile. Water can destroy its industry, so no owner is resolved until
/// the source's post-water type recheck. Later callbacks retain no Rust borrow.
/// Any reached water command uses the caller's live, independent stack result context.
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn tile_loop(
    tile: u32,
    log_x: u8,
    call: *mut crate::ground::Call,
    s: *const Services,
    c: *const ConstructionServices,
    l: *const LifecycleServices,
    t: *const TileServices,
    g: *const GenerationServices,
) {
    let (s, c, l, t) = unsafe { (&*s, &*c, &*l, &*t) };
    let map = crate::services::Map { log_x };
    if unsafe { (t.on_water)(tile) } {
        unsafe { crate::water::tree_industry_water_loop(tile, call, s.shared()) };
    }
    if (s.shared().tile_type)(tile) != MP_INDUSTRY {
        return;
    }
    let industry = unsafe { (t.industry)(tile) };
    unsafe {
        super::openttd_rust_industry_randomise_tile(
            tile,
            IndustryRandomTrigger::TileLoop.0,
            industry.shell.cast::<c_void>(),
            industry.owner,
            s,
            l,
        );
    };
    if !unsafe { (t.completed)(tile) } {
        bigger(tile, map, s, c, l, t);
        return;
    }
    if unsafe { ((*g).environment.editor)() } {
        return;
    }
    if unsafe { (t.transport)(tile) } {
        let industry = unsafe { (t.industry)(tile) };
        if !unsafe {
            super::openttd_rust_industry_animate_abi(
                industry.owner,
                (t.industry_index)(tile),
                IndustryAnimationTrigger::CargoDistributed.0,
                map.size_x(),
                s,
                l,
            ) != 0
        } {
            let gfx = unsafe { (t.gfx)(tile) };
            let next = unsafe { u16::from((*industrytype::tile(gfx)).anim_production) };
            if next != IndustryGfx::INDUSTRYTILE_NOANIM.0 {
                unsafe {
                    (t.reset_construction)(tile);
                    (t.set_completed)(tile);
                    (t.set_gfx)(tile, next);
                }
                dirty(tile, s);
                return;
            }
        }
    }
    if unsafe {
        super::openttd_rust_industry_animate_tile_abi(
            tile,
            IndustryAnimationTrigger::TileLoop.0,
            0,
            s,
            l,
        ) != 0
    } {
        return;
    }
    let gfx = unsafe { (t.gfx)(tile) };
    let next = unsafe { u16::from((*industrytype::tile(gfx)).anim_next) };
    if next != IndustryGfx::INDUSTRYTILE_NOANIM.0 {
        unsafe {
            (t.reset_construction)(tile);
            (t.set_gfx)(tile, next);
        };
        dirty(tile, s);
        return;
    }
    let mut gfx = unsafe { (t.gfx)(tile) };
    match gfx {
        GFX_COAL_MINE_TOWER_NOT_ANIMATED
        | GFX_COPPER_MINE_TOWER_NOT_ANIMATED
        | GFX_GOLD_MINE_TOWER_NOT_ANIMATED => {
            if (s.shared().tick_counter)() & 0x400 == 0 && chance(s, 1, 2) {
                gfx = match gfx {
                    GFX_COAL_MINE_TOWER_NOT_ANIMATED => GFX_COAL_MINE_TOWER_ANIMATED,
                    GFX_COPPER_MINE_TOWER_NOT_ANIMATED => GFX_COPPER_MINE_TOWER_ANIMATED,
                    _ => GFX_GOLD_MINE_TOWER_ANIMATED,
                };
                unsafe {
                    (t.set_gfx)(tile, gfx);
                    (t.set_frame)(tile, 0x80);
                    crate::animated_tile::openttd_rust_animation_add(tile, true);
                }
            }
        }
        GFX_OILWELL_NOT_ANIMATED => {
            if chance(s, 1, 6) {
                unsafe {
                    (t.set_gfx)(tile, GFX_OILWELL_ANIMATED_1);
                    (t.set_frame)(tile, 0);
                    crate::animated_tile::openttd_rust_animation_add(tile, true);
                }
            }
        }
        GFX_COAL_MINE_TOWER_ANIMATED
        | GFX_COPPER_MINE_TOWER_ANIMATED
        | GFX_GOLD_MINE_TOWER_ANIMATED => {
            if (s.shared().tick_counter)() & 0x400 == 0 {
                gfx = match gfx {
                    GFX_COAL_MINE_TOWER_ANIMATED => GFX_COAL_MINE_TOWER_NOT_ANIMATED,
                    GFX_COPPER_MINE_TOWER_ANIMATED => GFX_COPPER_MINE_TOWER_NOT_ANIMATED,
                    _ => GFX_GOLD_MINE_TOWER_NOT_ANIMATED,
                };
                unsafe {
                    (t.set_gfx)(tile, gfx);
                    (t.set_completed)(tile);
                    (c.set_construction_stage)(tile, INDUSTRY_COMPLETED as u8);
                    crate::animated_tile::openttd_rust_animation_delete(tile, false);
                }
                dirty(tile, s);
            }
        }
        GFX_POWERPLANT_SPARKS => {
            if chance(s, 1, 3) {
                sound(tile, SoundFx::SND_0C_POWER_STATION.into(), s);
                crate::animated_tile::openttd_rust_animation_add(tile, true);
            }
        }
        GFX_COPPER_MINE_CHIMNEY => {
            crate::effect::openttd_rust_effect_create_above(
                map.tile_x(tile).wrapping_mul(TILE_SIZE).wrapping_add(6) as i32,
                map.tile_y(tile).wrapping_mul(TILE_SIZE).wrapping_add(6) as i32,
                43,
                crate::effectvehicle_func::EV_COPPER_MINE_SMOKE,
            );
        }
        GFX_TOY_FACTORY => {
            let industry = unsafe { (t.industry)(tile) };
            let delivered =
                unsafe { ptr::read(ptr::addr_of!((*industry.owner).fields.was_cargo_delivered)) };
            if delivered != 0 {
                unsafe {
                    (*industry.owner).fields.was_cargo_delivered = 0;
                    (t.set_animation_loop)(tile, 0);
                    crate::animated_tile::openttd_rust_animation_add(tile, true);
                }
            }
        }
        GFX_BUBBLE_GENERATOR => bubble(tile, s, map),
        GFX_TOFFEE_QUARRY => crate::animated_tile::openttd_rust_animation_add(tile, true),
        GFX_SUGAR_MINE_SIEVE if chance(s, 1, 3) => {
            crate::animated_tile::openttd_rust_animation_add(tile, true);
        }
        _ => {}
    }
}

/// # Safety
/// Live industry tile, 64 native `CargoArray` entries and a live `CargoTypes` mask.
/// Immutable specs outlive the call. Resolver callbacks retain no owner borrow;
/// each final membership test reads the current accepted list at its source point.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_acceptance(
    tile: u32,
    acceptance: *mut u32,
    always_accepted: *mut u64,
    services: *const TileServices,
) {
    let t = unsafe { &*services };
    let gfx = unsafe { (t.gfx)(tile) };
    let spec = industrytype::tile(gfx);
    let industry = unsafe { (t.industry)(tile) };
    // Start with this tile specification's cargoes and acceptance.
    let mut info = unsafe {
        TileAcceptance {
            cargo: (*spec).accepts_cargo,
            amount: (*spec).acceptance,
            accepts_all: IndustryTileSpecialFlags((*spec).special_flags)
                .test(IndustryTileSpecialFlag::AcceptsAllCargo),
            accepts_callback: IndustryTileCallbackMasks((*spec).callback_mask)
                .test(IndustryTileCallbackMask::AcceptCargo),
            amount_callback: IndustryTileCallbackMasks((*spec).callback_mask)
                .test(IndustryTileCallbackMask::CargoAcceptance),
        }
    };
    if info.accepts_all {
        // Copy accepted cargoes from the industry itself. Insert a missing cargo
        // into an invalid slot; if no slot exists, give up on that cargo.
        for n in 0..super::accepted_count(industry.owner) {
            let cargo = unsafe { (*super::accepted(industry.owner, n)).cargo };
            let Some(slot) = info
                .cargo
                .iter()
                .position(|&c| c == cargo)
                .or_else(|| info.cargo.iter().position(|&c| c == INVALID_CARGO))
            else {
                continue;
            };
            info.cargo[slot] = cargo;
            info.amount[slot] = info.amount[slot].wrapping_add(8);
        }
    }
    // A successful accepts callback replaces the complete accepts list.
    if info.accepts_callback {
        let result =
            unsafe { (t.callback)(tile, gfx, CallbackID::CBID_INDTILE_ACCEPT_CARGO.0, 0, 0) };
        if result != CALLBACK_FAILED {
            info.cargo.fill(INVALID_CARGO);
            for n in 0..INDUSTRY_ORIGINAL_NUM_INPUTS {
                info.cargo[n] = crate::cargo_registry::openttd_rust_cargo_translate(
                    ((result >> (n * 5)) & 31) as u8,
                    unsafe { (*spec).grf_prop.grffile },
                    false,
                );
            }
        }
    }
    // A successful acceptance callback replaces the complete acceptance list.
    if info.amount_callback {
        let result =
            unsafe { (t.callback)(tile, gfx, CallbackID::CBID_INDTILE_CARGO_ACCEPTANCE.0, 0, 0) };
        if result != CALLBACK_FAILED {
            info.amount.fill(0);
            for n in 0..INDUSTRY_ORIGINAL_NUM_INPUTS {
                info.amount[n] = ((result >> (n * 4)) & 15) as i8;
            }
        }
    }
    for n in 0..INDUSTRY_NUM_INPUTS {
        let cargo = info.cargo[n];
        let amount = info.amount[n];
        // Work only with valid cargoes.
        if cargo == INVALID_CARGO || amount <= 0 {
            continue;
        }
        // Add accepted cargo.
        unsafe {
            let entry = acceptance.add(usize::from(cargo));
            *entry = (*entry).wrapping_add(amount as u32);
        }
        // Consider the always-accepted bit only if it is not already set.
        let bit = 1_u64.wrapping_shl(u32::from(cargo));
        if unsafe { *always_accepted } & bit != 0 {
            continue;
        }
        // The industry's own acceptance prevents the always-accepted bit.
        if (0..super::accepted_count(industry.owner))
            .any(|slot| unsafe { (*super::accepted(industry.owner, slot)).cargo == cargo })
        {
            continue;
        }
        // A cargo the industry itself does not accept is always accepted by the tile.
        unsafe { *always_accepted |= bit };
    }
}

#[repr(C)]
pub struct TilePolicyServices {
    pub current_company: unsafe extern "C" fn() -> u8,
    pub magic_bulldozer: unsafe extern "C" fn() -> bool,
    pub object_in_way: unsafe extern "C" fn(*mut IndustryCost, *const SpecHandle),
    pub close_ai: unsafe extern "C" fn(*mut ShellHandle),
    pub close_game: unsafe extern "C" fn(*mut ShellHandle),
    pub destroy: unsafe extern "C" fn(*mut ShellHandle),
    pub tile_max_z: unsafe extern "C" fn(u32) -> i32,
    pub command_checks: *const crate::services::CommandServices,
}
/// # Safety
/// Live tile and native stack result. Event and destruction callbacks may reenter;
/// immutable specs remain live after owner destruction, with no owner borrow held.
pub(crate) unsafe fn openttd_rust_industry_tile_clear(
    out: *mut IndustryCost,
    tile: u32,
    flags: u16,
    tiles: *const TileServices,
    policy: *const TilePolicyServices,
    costs: *const IndustryCostServices,
    generation: *const GenerationServices,
) {
    let (t, p, c, g) = unsafe { (&*tiles, &*policy, &*costs, &*generation) };
    let industry = unsafe { (t.industry)(tile) };
    let kind = unsafe { (*industry.owner).metadata.kind };
    let spec = industrytype::industry(u16::from(kind));
    if (unsafe { (p.current_company)() } != Owner::WATER.0
        && !(g.environment.editor)()
        && !unsafe { (p.magic_bulldozer)() })
        || DoCommandFlags(flags).test(DoCommandFlag::Auto)
        || (unsafe { (p.current_company)() } == Owner::WATER.0
            && (IndustryBehaviours(unsafe { (*spec).behaviour })
                .test(IndustryBehaviour::BuiltOnWater)
                || unsafe { (*industrytype::tile((t.gfx)(tile))).slopes_refused } & 32 != 0))
    {
        if DoCommandFlags(flags).test(DoCommandFlag::Auto) {
            unsafe { (p.object_in_way)(out, spec) };
        } else {
            unsafe { (c.error)(out, crate::strings_type::INVALID_STRING_ID) };
        }
        return;
    }
    if DoCommandFlags(flags).test(DoCommandFlag::Execute) {
        unsafe {
            (p.close_ai)(industry.shell);
            (p.close_game)(industry.shell);
            (p.destroy)(industry.shell);
        }
    }
    let removal = unsafe { super::removal_cost((*spec).removal_cost_multiplier) };
    unsafe { (c.expense_money)(out, crate::economy_type::EXPENSES_CONSTRUCTION, removal) };
}
/// # Safety
/// Native complete cost sink and live tile. The resolver can reenter; no owner or
/// map reference survives it. `LandscapeClear` returns its original full cost.
#[allow(clippy::too_many_arguments)]
pub(crate) unsafe fn openttd_rust_industry_tile_terraform(
    out: *mut IndustryCost,
    tile: u32,
    flags: u16,
    z_new: i32,
    slope_new: u8,
    shared: *const crate::services::Services,
    tiles: *const TileServices,
    policy: *const TilePolicyServices,
    costs: *const IndustryCostServices,
) {
    let (shared, t, p, c) = unsafe { (&*shared, &*tiles, &*policy, &*costs) };
    if (shared.autoslope)() {
        let old_slope = crate::map_func::World::new(shared).tile_slope(tile);
        if !is_steep_slope(old_slope)
            && !is_steep_slope(slope_new)
            && unsafe { (p.tile_max_z)(tile) } == z_new.wrapping_add(get_slope_max_z(slope_new))
        {
            let gfx = unsafe { (t.gfx)(tile) };
            let spec = industrytype::tile(gfx);
            let allowed = if unsafe {
                IndustryTileCallbackMasks((*spec).callback_mask)
                    .test(IndustryTileCallbackMask::Autoslope)
            } {
                let result =
                    unsafe { (t.callback)(tile, gfx, CallbackID::CBID_INDTILE_AUTOSLOPE.0, 0, 0) };
                result == CALLBACK_FAILED
                    || !(shared.grf_boolean_callback)(
                        unsafe { (*spec).grf_prop.grffile },
                        CallbackID::CBID_INDTILE_AUTOSLOPE.0,
                        result,
                    )
            } else {
                true
            };
            if allowed {
                let foundation = crate::company::price(PR_BUILD_FOUNDATION);
                unsafe {
                    (c.expense_money)(out, crate::economy_type::EXPENSES_CONSTRUCTION, foundation)
                };
                return;
            }
        }
    }
    crate::landscape_cmd::clear_command(unsafe { &*p.command_checks }, out.cast(), tile, flags);
}
