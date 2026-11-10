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
    ConstructionServices, Industry, IndustryCost, IndustryCostServices, LifecycleServices,
    Services, SpecHandle,
};
use crate::bitmath_func::{gb, has_bit, set_bit64};
use crate::cargo_type::{CargoArray, CargoType, CargoTypes};
use crate::command_type::{DoCommandFlag, DoCommandFlags};
use crate::company_type::{CompanyID, Owner};
use crate::economy_type::{ExpensesType, Price};
use crate::effectvehicle_func::{EV_BUBBLE, EV_CHIMNEY_SMOKE, EV_COPPER_MINE_SMOKE};
use crate::industry_type::{
    INDUSTRY_COMPLETED, INDUSTRY_NUM_INPUTS, INDUSTRY_ORIGINAL_NUM_INPUTS,
    IndustryAnimationTrigger, IndustryGfx, IndustryRandomTrigger,
};
use crate::industrytype::{
    IndustryBehaviour, IndustryBehaviours, IndustryTileSpec, IndustryTileSpecialFlag,
    IndustryTileSpecialFlags, get_industry_spec, get_industry_tile_spec,
};
use crate::newgrf_animation_type::AnimationStatus;
use crate::newgrf_callbacks::CALLBACK_FAILED;
use crate::newgrf_callbacks::{CallbackID, IndustryTileCallbackMask, IndustryTileCallbackMasks};
use crate::slope_func::{get_slope_max_z, is_steep_slope};
use crate::slope_type::Slope;
use crate::sound_type::{SoundFx, SoundID};
use crate::strings_type::StringID;
use crate::tile_type::{TILE_SIZE, TileIndex, TileType};
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
impl TileServices {
    /// `GetIndustryGfx`.
    fn get_industry_gfx(&self, tile: TileIndex) -> IndustryGfx {
        IndustryGfx(unsafe { (self.gfx)(tile.0) })
    }
    /// `SetIndustryGfx`.
    fn set_industry_gfx(&self, tile: TileIndex, gfx: IndustryGfx) {
        unsafe { (self.set_gfx)(tile.0, gfx.0) };
    }
    /// `GetAnimationFrame`.
    fn get_animation_frame(&self, tile: TileIndex) -> u8 {
        unsafe { (self.frame)(tile.0) }
    }
    /// `SetAnimationFrame`.
    fn set_animation_frame(&self, tile: TileIndex, frame: u8) {
        unsafe { (self.set_frame)(tile.0, frame) };
    }
    /// `SetIndustryCompleted`.
    fn set_industry_completed(&self, tile: TileIndex) {
        unsafe { (self.set_completed)(tile.0) };
    }
    /// `ResetIndustryConstructionStage`.
    fn reset_industry_construction_stage(&self, tile: TileIndex) {
        unsafe { (self.reset_construction)(tile.0) };
    }
    /// `SetIndustryAnimationLoop`.
    fn set_industry_animation_loop(&self, tile: TileIndex, count: u8) {
        unsafe { (self.set_animation_loop)(tile.0, count) };
    }
    /// `GetIndustryTileCallback` for `tile` with no parameters.
    fn tile_callback(&self, tile: TileIndex, gfx: IndustryGfx, callback: CallbackID) -> u16 {
        unsafe { (self.callback)(tile.0, gfx.0, callback.0, 0, 0) }
    }
}
fn ambient(s: &Services) -> bool {
    unsafe { s.shared().settings().ambient.read() }
}
fn snd_play_tile_fx(sound: SoundID, tile: TileIndex, s: &Services) {
    unsafe { (s.play_sound)(sound.0, tile.0) };
}
/// `if (_settings_client.sound.ambient) SndPlayTileFx(sound, tile)`.
fn ambient_sound(tile: TileIndex, sound: SoundFx, s: &Services) {
    if ambient(s) {
        snd_play_tile_fx(sound.into(), tile, s);
    }
}
fn mark_tile_dirty_by_tile(tile: TileIndex, s: &Services) {
    (s.shared().mark_dirty)(tile.0);
}
fn add_animated_tile(tile: TileIndex) {
    crate::animated_tile::openttd_rust_animation_add(tile.0, true);
}
fn delete_animated_tile(tile: TileIndex) {
    crate::animated_tile::openttd_rust_animation_delete(tile.0, false);
}
fn tick_counter(s: &Services) -> u64 {
    (s.shared().tick_counter)()
}
/// `AnimateSugarSieve`.
fn animate_sugar_sieve(tile: TileIndex, s: &Services, t: &TileServices) {
    let mut m = t.get_animation_frame(tile).wrapping_add(1);
    if ambient(s) {
        match m & 7 {
            2 => snd_play_tile_fx(SoundFx::SND_2D_SUGAR_MINE_1.into(), tile, s),
            6 => snd_play_tile_fx(SoundFx::SND_29_SUGAR_MINE_2.into(), tile, s),
            _ => {}
        }
    }
    if m >= 96 {
        m = 0;
        delete_animated_tile(tile);
    }
    t.set_animation_frame(tile, m);
    mark_tile_dirty_by_tile(tile, s);
}
/// `AnimateToffeeQuarry`.
fn animate_toffee_quarry(tile: TileIndex, s: &Services, t: &TileServices) {
    let mut m = t.get_animation_frame(tile);
    if unsafe { (t.toffee_offset)(m) } == 0xff {
        ambient_sound(tile, SoundFx::SND_30_TOFFEE_QUARRY, s);
    }
    m = m.wrapping_add(1);
    if m >= 70 {
        m = 0;
        delete_animated_tile(tile);
    }
    t.set_animation_frame(tile, m);
    mark_tile_dirty_by_tile(tile, s);
}
/// `AnimateBubbleCatcher`.
fn animate_bubble_catcher(tile: TileIndex, s: &Services, t: &TileServices) {
    let mut m = t.get_animation_frame(tile).wrapping_add(1);
    if m >= 40 {
        m = 0;
        delete_animated_tile(tile);
    }
    t.set_animation_frame(tile, m);
    mark_tile_dirty_by_tile(tile, s);
}
/// `AnimatePowerPlantSparks`.
fn animate_power_plant_sparks(tile: TileIndex, s: &Services, t: &TileServices) {
    let m = t.get_animation_frame(tile);
    if m == 6 {
        t.set_animation_frame(tile, 0);
        delete_animated_tile(tile);
    } else {
        t.set_animation_frame(tile, m.wrapping_add(1));
    }
    mark_tile_dirty_by_tile(tile, s);
}
/// `AnimateToyFactory`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "AnimateToyFactory keeps the original's implicit C++ integer conversions"
)]
fn animate_toy_factory(tile: TileIndex, s: &Services, t: &TileServices) {
    let mut m = t.get_animation_frame(tile).wrapping_add(1);
    match m {
        1 => ambient_sound(tile, SoundFx::SND_2C_TOY_FACTORY_1, s),
        23 => ambient_sound(tile, SoundFx::SND_2B_TOY_FACTORY_2, s),
        28 => ambient_sound(tile, SoundFx::SND_2A_TOY_FACTORY_3, s),
        _ => {
            if m >= 50 {
                let mut n = u32::from(unsafe { (t.animation_loop)(tile.0) }).wrapping_add(1);
                m = 0;
                if n >= 8 {
                    n = 0;
                    delete_animated_tile(tile);
                }
                t.set_industry_animation_loop(tile, n as u8);
            }
        }
    }
    t.set_animation_frame(tile, m);
    mark_tile_dirty_by_tile(tile, s);
}
/// `AnimatePlasticFountain`.
fn animate_plastic_fountain(tile: TileIndex, gfx: IndustryGfx, s: &Services, t: &TileServices) {
    let gfx = if gfx < IndustryGfx::GFX_PLASTIC_FOUNTAIN_ANIMATED_8 {
        IndustryGfx(gfx.0.wrapping_add(1))
    } else {
        IndustryGfx::GFX_PLASTIC_FOUNTAIN_ANIMATED_1
    };
    t.set_industry_gfx(tile, gfx);
    mark_tile_dirty_by_tile(tile, s);
}
/// `AnimateOilWell`.
fn animate_oil_well(
    tile: TileIndex,
    mut gfx: IndustryGfx,
    svc: &Services,
    cons: &ConstructionServices,
    tiles: &TileServices,
) {
    let b = crate::random_func::chance16(svc.shared(), 1, 7);
    let mut m = tiles.get_animation_frame(tile).wrapping_add(1);
    // `m == 4 && (m = 0, ++gfx) == GFX_OILWELL_ANIMATED_3 + 1 && (gfx = GFX_OILWELL_ANIMATED_1, b)`
    let last = m == 4
        && {
            m = 0;
            gfx = IndustryGfx(gfx.0.wrapping_add(1));
            gfx.0 == IndustryGfx::GFX_OILWELL_ANIMATED_3.0 + 1
        }
        && {
            gfx = IndustryGfx::GFX_OILWELL_ANIMATED_1;
            b
        };
    if last {
        tiles.set_industry_gfx(tile, IndustryGfx::GFX_OILWELL_NOT_ANIMATED);
        unsafe { (cons.set_construction_stage)(tile.0, 3) };
        delete_animated_tile(tile);
    } else {
        tiles.set_animation_frame(tile, m);
        tiles.set_industry_gfx(tile, gfx);
    }
    mark_tile_dirty_by_tile(tile, svc);
}
/// `AnimateMineTower`.
fn animate_mine_tower(tile: TileIndex, s: &Services, t: &TileServices) {
    let state = (tick_counter(s) & 0x7ff) as i32 - 0x400;
    if state < 0 {
        return;
    }
    if state < 0x1a0 {
        if !(0x20..0x180).contains(&state) {
            let m = t.get_animation_frame(tile);
            if m & 0x40 == 0 {
                t.set_animation_frame(tile, m | 0x40);
                ambient_sound(tile, SoundFx::SND_0B_MINE, s);
            }
            if state & 7 != 0 {
                return;
            }
        } else if state & 3 != 0 {
            return;
        }
        let mut m = t.get_animation_frame(tile).wrapping_add(1) | 0x40;
        if m > 0xc2 {
            m = 0xc0;
        }
        t.set_animation_frame(tile, m);
        mark_tile_dirty_by_tile(tile, s);
    } else if (0x200..0x3a0).contains(&state) {
        let i = if (0x220..0x380).contains(&state) {
            3
        } else {
            7
        };
        if state & i != 0 {
            return;
        }
        let mut m = (t.get_animation_frame(tile) & 0xbf).wrapping_sub(1);
        if m < 0x80 {
            m = 0x82;
        }
        t.set_animation_frame(tile, m);
        mark_tile_dirty_by_tile(tile, s);
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
    unsafe { animate_tile_industry(TileIndex(tile), &*s, &*c, &*t) };
}
/// `AnimateTile_Industry`.
pub(crate) fn animate_tile_industry(
    tile: TileIndex,
    s: &Services,
    c: &ConstructionServices,
    t: &TileServices,
) {
    let gfx = t.get_industry_gfx(tile);
    if unsafe { (*get_industry_tile_spec(gfx)).animation.status } != AnimationStatus::NoAnimation.0
    {
        crate::animated_tile::openttd_rust_animate_industry(tile.0);
        return;
    }
    match gfx {
        IndustryGfx::GFX_SUGAR_MINE_SIEVE if tick_counter(s) & 1 == 0 => {
            animate_sugar_sieve(tile, s, t);
        }
        IndustryGfx::GFX_TOFFEE_QUARRY if tick_counter(s) & 3 == 0 => {
            animate_toffee_quarry(tile, s, t);
        }
        IndustryGfx::GFX_BUBBLE_CATCHER if tick_counter(s) & 1 == 0 => {
            animate_bubble_catcher(tile, s, t);
        }
        IndustryGfx::GFX_POWERPLANT_SPARKS if tick_counter(s) & 3 == 0 => {
            animate_power_plant_sparks(tile, s, t);
        }
        IndustryGfx::GFX_TOY_FACTORY if tick_counter(s) & 1 == 0 => {
            animate_toy_factory(tile, s, t);
        }
        _ if (IndustryGfx::GFX_PLASTIC_FOUNTAIN_ANIMATED_1
            ..=IndustryGfx::GFX_PLASTIC_FOUNTAIN_ANIMATED_8)
            .contains(&gfx) =>
        {
            if tick_counter(s) & 3 == 0 {
                animate_plastic_fountain(tile, gfx, s, t);
            }
        }
        _ if (IndustryGfx::GFX_OILWELL_ANIMATED_1..=IndustryGfx::GFX_OILWELL_ANIMATED_3)
            .contains(&gfx) =>
        {
            if tick_counter(s) & 7 == 0 {
                animate_oil_well(tile, gfx, s, c, t);
            }
        }
        IndustryGfx::GFX_COAL_MINE_TOWER_ANIMATED
        | IndustryGfx::GFX_COPPER_MINE_TOWER_ANIMATED
        | IndustryGfx::GFX_GOLD_MINE_TOWER_ANIMATED => animate_mine_tower(tile, s, t),
        _ => {}
    }
}
/// `CreateChimneySmoke`.
#[allow(
    clippy::cast_possible_wrap,
    reason = "CreateChimneySmoke keeps the original's implicit C++ integer conversions"
)]
fn create_chimney_smoke(tile: TileIndex, map: crate::services::Map, t: &TileServices) {
    let x = map.tile_x(tile.0).wrapping_mul(TILE_SIZE);
    let y = map.tile_y(tile.0).wrapping_mul(TILE_SIZE);
    let z = unsafe { (t.pixel_max_z)(tile.0) };
    crate::effect::openttd_rust_effect_create(
        x.wrapping_add(15) as i32,
        y.wrapping_add(14) as i32,
        z.wrapping_add(59),
        EV_CHIMNEY_SMOKE,
    );
}
/// `MakeIndustryTileBigger`.
fn make_industry_tile_bigger(
    tile: TileIndex,
    map: crate::services::Map,
    s: &Services,
    c: &ConstructionServices,
    l: &LifecycleServices,
    t: &TileServices,
) {
    let cnt = unsafe { (t.construction_counter)(tile.0) }.wrapping_add(1);
    if cnt != 4 {
        unsafe { (c.set_construction_counter)(tile.0, cnt) };
        return;
    }
    let stage = unsafe { (t.construction_stage)(tile.0) }.wrapping_add(1);
    unsafe {
        (c.set_construction_counter)(tile.0, 0);
        (c.set_construction_stage)(tile.0, stage);
        // `TriggerIndustryTileAnimation_ConstructionStageChanged(tile, false)`.
        super::trigger_industry_tile_animation(
            tile,
            IndustryAnimationTrigger::ConstructionStageChanged,
            0,
            s,
            l,
        );
        if i32::from(stage) == INDUSTRY_COMPLETED {
            t.set_industry_completed(tile);
        }
    }
    mark_tile_dirty_by_tile(tile, s);
    if !unsafe { (t.completed)(tile.0) } {
        return;
    }
    let gfx = t.get_industry_gfx(tile);
    if gfx >= IndustryGfx::NEW_INDUSTRYTILEOFFSET {
        // New industries are already animated on construction.
        return;
    }
    match gfx {
        IndustryGfx::GFX_POWERPLANT_CHIMNEY => create_chimney_smoke(tile, map, t),
        IndustryGfx::GFX_OILRIG_1 => {
            // Do not require an industry tile to be after the first two GFX_OILRIG_1 tiles
            // (like the default oil rig). Do a proper check to ensure the tiles belong to
            // the same industry and based on that build the oil rig's station.
            let other = TileIndex(tile.0.wrapping_add(map.diff(0, 1)));
            if TileType((s.shared().tile_type)(other.0)) == TileType::MP_INDUSTRY
                && t.get_industry_gfx(other) == IndustryGfx::GFX_OILRIG_1
                && unsafe { (t.industry_index)(tile.0) == (t.industry_index)(other.0) }
            {
                unsafe { (t.build_oilrig)(tile.0) };
            }
        }
        IndustryGfx::GFX_TOY_FACTORY
        | IndustryGfx::GFX_BUBBLE_CATCHER
        | IndustryGfx::GFX_TOFFEE_QUARRY => {
            t.set_animation_frame(tile, 0);
            t.set_industry_animation_loop(tile, 0);
        }
        _ if (IndustryGfx::GFX_PLASTIC_FOUNTAIN_ANIMATED_1
            ..=IndustryGfx::GFX_PLASTIC_FOUNTAIN_ANIMATED_8)
            .contains(&gfx) =>
        {
            crate::animated_tile::openttd_rust_animation_add(tile.0, false);
        }
        _ => {}
    }
}
/// `TileLoopIndustry_BubbleGenerator`.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    reason = "TileLoopIndustry_BubbleGenerator keeps the original's implicit C++ integer conversions"
)]
fn tile_loop_industry_bubble_generator(tile: TileIndex, s: &Services, map: crate::services::Map) {
    const BUBBLE_SPAWN_LOCATION: [[i8; 4]; 3] =
        [[11, 0, -4, -14], [-4, -10, -4, 1], [49, 59, 60, 65]];
    ambient_sound(tile, SoundFx::SND_2E_BUBBLE_GENERATOR, s);
    let dir = (s.shared().random() & 3) as usize;
    let v = crate::effect::openttd_rust_effect_create_above(
        map.tile_x(tile.0)
            .wrapping_mul(TILE_SIZE)
            .wrapping_add(i32::from(BUBBLE_SPAWN_LOCATION[0][dir]) as u32) as i32,
        map.tile_y(tile.0)
            .wrapping_mul(TILE_SIZE)
            .wrapping_add(i32::from(BUBBLE_SPAWN_LOCATION[1][dir]) as u32) as i32,
        i32::from(BUBBLE_SPAWN_LOCATION[2][dir]),
        EV_BUBBLE,
    );
    if !v.is_null() {
        crate::effect::set_substate(v, dir as u8);
    }
}
/// `TileLoop_Industry`, with the tile-loop batch's industry tables.
///
/// # Safety
/// Live map tile. Water can destroy its industry, so no owner is resolved until
/// the source's post-water type recheck. Later callbacks retain no Rust borrow.
/// Any reached water command uses the caller's live, independent stack result context.
#[allow(
    clippy::cast_possible_wrap,
    reason = "TileLoop_Industry keeps the original's implicit C++ integer conversions"
)]
#[allow(
    clippy::too_many_lines,
    reason = "TileLoop_Industry is ported as the original's single body; its length is tracked in the Clippy debt ledger"
)]
pub(crate) unsafe fn tile_loop(
    tile: TileIndex,
    map: crate::services::Map,
    call: *mut crate::ground::Call,
    services: &crate::tile_loop::IndustryServices,
) {
    let (prod, cons) = unsafe { (&*services.production, &*services.construction) };
    let (life, tiles) = unsafe { (&*services.lifecycle, &*services.tiles) };
    let generation = unsafe { &*services.generation };
    if unsafe { (tiles.on_water)(tile.0) } {
        unsafe { crate::water::tree_industry_water_loop(tile.0, call, prod.shared()) };
    }
    // Normally this doesn't happen, but if an industry NewGRF is removed an industry that
    // was previously build on water can now be flooded. If this happens the tile is no
    // longer an industry tile after returning from TileLoop_Water.
    if TileType((prod.shared().tile_type)(tile.0)) != TileType::MP_INDUSTRY {
        return;
    }
    let industry = unsafe { (tiles.industry)(tile.0) };
    unsafe {
        super::trigger_industry_tile_randomisation(
            tile,
            IndustryRandomTrigger::TileLoop,
            industry.shell.cast::<c_void>(),
            industry.owner,
            prod,
            life,
        );
    };
    if !unsafe { (tiles.completed)(tile.0) } {
        make_industry_tile_bigger(tile, map, prod, cons, life, tiles);
        return;
    }
    if (generation.environment.editor)() {
        return;
    }
    if unsafe { (tiles.transport)(tile.0) } {
        let industry = unsafe { (tiles.industry)(tile.0) };
        if !unsafe {
            super::trigger_industry_animation(
                industry.owner,
                crate::industry_type::IndustryID((tiles.industry_index)(tile.0)),
                IndustryAnimationTrigger::CargoDistributed,
                map.size_x(),
                prod,
                life,
            )
        } {
            let gfx = tiles.get_industry_gfx(tile);
            let newgfx = IndustryGfx(u16::from(unsafe {
                (*get_industry_tile_spec(gfx)).anim_production
            }));
            if newgfx != IndustryGfx::INDUSTRYTILE_NOANIM {
                tiles.reset_industry_construction_stage(tile);
                tiles.set_industry_completed(tile);
                tiles.set_industry_gfx(tile, newgfx);
                mark_tile_dirty_by_tile(tile, prod);
                return;
            }
        }
    }
    if unsafe {
        super::trigger_industry_tile_animation(
            tile,
            IndustryAnimationTrigger::TileLoop,
            0,
            prod,
            life,
        )
    } {
        return;
    }
    let gfx = tiles.get_industry_gfx(tile);
    let newgfx = IndustryGfx(u16::from(unsafe {
        (*get_industry_tile_spec(gfx)).anim_next
    }));
    if newgfx != IndustryGfx::INDUSTRYTILE_NOANIM {
        tiles.reset_industry_construction_stage(tile);
        tiles.set_industry_gfx(tile, newgfx);
        mark_tile_dirty_by_tile(tile, prod);
        return;
    }
    let mut gfx = tiles.get_industry_gfx(tile);
    match gfx {
        IndustryGfx::GFX_COAL_MINE_TOWER_NOT_ANIMATED
        | IndustryGfx::GFX_COPPER_MINE_TOWER_NOT_ANIMATED
        | IndustryGfx::GFX_GOLD_MINE_TOWER_NOT_ANIMATED => {
            if tick_counter(prod) & 0x400 == 0 && crate::random_func::chance16(prod.shared(), 1, 2)
            {
                gfx = match gfx {
                    IndustryGfx::GFX_COAL_MINE_TOWER_NOT_ANIMATED => {
                        IndustryGfx::GFX_COAL_MINE_TOWER_ANIMATED
                    }
                    IndustryGfx::GFX_COPPER_MINE_TOWER_NOT_ANIMATED => {
                        IndustryGfx::GFX_COPPER_MINE_TOWER_ANIMATED
                    }
                    _ => IndustryGfx::GFX_GOLD_MINE_TOWER_ANIMATED,
                };
                tiles.set_industry_gfx(tile, gfx);
                tiles.set_animation_frame(tile, 0x80);
                add_animated_tile(tile);
            }
        }
        IndustryGfx::GFX_OILWELL_NOT_ANIMATED => {
            if crate::random_func::chance16(prod.shared(), 1, 6) {
                tiles.set_industry_gfx(tile, IndustryGfx::GFX_OILWELL_ANIMATED_1);
                tiles.set_animation_frame(tile, 0);
                add_animated_tile(tile);
            }
        }
        IndustryGfx::GFX_COAL_MINE_TOWER_ANIMATED
        | IndustryGfx::GFX_COPPER_MINE_TOWER_ANIMATED
        | IndustryGfx::GFX_GOLD_MINE_TOWER_ANIMATED => {
            if tick_counter(prod) & 0x400 == 0 {
                gfx = match gfx {
                    IndustryGfx::GFX_COAL_MINE_TOWER_ANIMATED => {
                        IndustryGfx::GFX_COAL_MINE_TOWER_NOT_ANIMATED
                    }
                    IndustryGfx::GFX_COPPER_MINE_TOWER_ANIMATED => {
                        IndustryGfx::GFX_COPPER_MINE_TOWER_NOT_ANIMATED
                    }
                    _ => IndustryGfx::GFX_GOLD_MINE_TOWER_NOT_ANIMATED,
                };
                tiles.set_industry_gfx(tile, gfx);
                tiles.set_industry_completed(tile);
                unsafe { (cons.set_construction_stage)(tile.0, 3) };
                delete_animated_tile(tile);
                mark_tile_dirty_by_tile(tile, prod);
            }
        }
        IndustryGfx::GFX_POWERPLANT_SPARKS => {
            if crate::random_func::chance16(prod.shared(), 1, 3) {
                ambient_sound(tile, SoundFx::SND_0C_POWER_STATION, prod);
                add_animated_tile(tile);
            }
        }
        IndustryGfx::GFX_COPPER_MINE_CHIMNEY => {
            crate::effect::openttd_rust_effect_create_above(
                map.tile_x(tile.0).wrapping_mul(TILE_SIZE).wrapping_add(6) as i32,
                map.tile_y(tile.0).wrapping_mul(TILE_SIZE).wrapping_add(6) as i32,
                43,
                EV_COPPER_MINE_SMOKE,
            );
        }
        IndustryGfx::GFX_TOY_FACTORY => {
            let i = unsafe { (tiles.industry)(tile.0) };
            let delivered =
                unsafe { ptr::read(ptr::addr_of!((*i.owner).fields.was_cargo_delivered)) };
            if delivered != 0 {
                unsafe { (*i.owner).fields.was_cargo_delivered = 0 };
                tiles.set_industry_animation_loop(tile, 0);
                add_animated_tile(tile);
            }
        }
        IndustryGfx::GFX_BUBBLE_GENERATOR => tile_loop_industry_bubble_generator(tile, prod, map),
        IndustryGfx::GFX_TOFFEE_QUARRY => add_animated_tile(tile),
        IndustryGfx::GFX_SUGAR_MINE_SIEVE if crate::random_func::chance16(prod.shared(), 1, 3) => {
            add_animated_tile(tile);
        }
        _ => {}
    }
}

/// # Safety
/// Live industry tile, 64 native `CargoArray` entries and a live `CargoTypes` mask.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_acceptance(
    tile: u32,
    acceptance: *mut u32,
    always_accepted: *mut u64,
    services: *const TileServices,
) {
    unsafe {
        add_accepted_cargo_industry(
            TileIndex(tile),
            &mut *acceptance.cast::<CargoArray>(),
            &mut *always_accepted.cast::<CargoTypes>(),
            &*services,
        );
    }
}
/// `AddAcceptedCargo_Industry`.
///
/// # Safety
/// Live industry tile; immutable specs outlive the call. Resolver callbacks retain no
/// owner borrow; each final membership test reads the current accepted list at its
/// source point.
#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "AddAcceptedCargo_Industry keeps the original's implicit C++ integer conversions"
)]
pub(crate) unsafe fn add_accepted_cargo_industry(
    tile: TileIndex,
    acceptance: &mut CargoArray,
    always_accepted: &mut CargoTypes,
    t: &TileServices,
) {
    let gfx = t.get_industry_gfx(tile);
    let itspec = get_industry_tile_spec(gfx);
    let ind = unsafe { (t.industry)(tile.0) };
    // Starting point for acceptance.
    let mut info = unsafe {
        TileAcceptance {
            cargo: (*itspec).accepts_cargo,
            amount: (*itspec).acceptance,
            accepts_all: IndustryTileSpecialFlags((*itspec).special_flags)
                .test(IndustryTileSpecialFlag::AcceptsAllCargo),
            accepts_callback: IndustryTileCallbackMasks((*itspec).callback_mask)
                .test(IndustryTileCallbackMask::AcceptCargo),
            amount_callback: IndustryTileCallbackMasks((*itspec).callback_mask)
                .test(IndustryTileCallbackMask::CargoAcceptance),
        }
    };
    if info.accepts_all {
        // Copy all accepted cargoes from industry itself.
        for n in 0..super::accepted_count(ind.owner) {
            let cargo = CargoType(unsafe { (*super::accepted(ind.owner, n)).cargo });
            // Not found, insert
            let Some(pos) = info.cargo.iter().position(|&c| c == cargo.0).or_else(|| {
                info.cargo
                    .iter()
                    .position(|&c| c == CargoType::INVALID_CARGO.0)
            }) else {
                // nowhere to place, give up on this one
                continue;
            };
            info.cargo[pos] = cargo.0;
            info.amount[pos] = info.amount[pos].wrapping_add(8);
        }
    }
    if info.accepts_callback {
        // Try callback for accepts list, if success override all existing accepts.
        let res = t.tile_callback(tile, gfx, CallbackID::CBID_INDTILE_ACCEPT_CARGO);
        if res != CALLBACK_FAILED {
            info.cargo.fill(CargoType::INVALID_CARGO.0);
            for i in 0..INDUSTRY_ORIGINAL_NUM_INPUTS {
                info.cargo[i] = crate::cargo_registry::openttd_rust_cargo_translate(
                    gb(u32::from(res), (i * 5) as u8, 5) as u8,
                    unsafe { (*itspec).grf_prop.grffile },
                    false,
                );
            }
        }
    }
    if info.amount_callback {
        // Try callback for acceptance list, if success override all existing acceptance.
        let res = t.tile_callback(tile, gfx, CallbackID::CBID_INDTILE_CARGO_ACCEPTANCE);
        if res != CALLBACK_FAILED {
            info.amount.fill(0);
            for i in 0..INDUSTRY_ORIGINAL_NUM_INPUTS {
                info.amount[i] = gb(u32::from(res), (i * 4) as u8, 4) as i8;
            }
        }
    }
    for i in 0..INDUSTRY_NUM_INPUTS {
        let cargo = CargoType(info.cargo[i]);
        let amount = info.amount[i];
        // Work only with valid cargoes.
        if cargo == CargoType::INVALID_CARGO || amount <= 0 {
            continue;
        }
        // Add accepted cargo.
        let entry = &mut acceptance[usize::from(cargo.0)];
        *entry = entry.wrapping_add(amount as u32);
        // Maybe set 'always accepted' bit (if it's not set already).
        let bit = 1_u64.wrapping_shl(u32::from(cargo.0));
        if always_accepted.0 & bit != 0 {
            continue;
        }
        // Test whether the industry itself accepts the cargo type.
        if (0..super::accepted_count(ind.owner))
            .any(|slot| unsafe { (*super::accepted(ind.owner, slot)).cargo == cargo.0 })
        {
            continue;
        }
        // If the industry itself doesn't accept this cargo, set 'always accepted' bit.
        set_bit64(&mut always_accepted.0, cargo.0);
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
/// `ClearTile_Industry`.
///
/// # Safety
/// Live tile and native stack result. Event and destruction callbacks may reenter;
/// immutable specs remain live after owner destruction, with no owner borrow held.
pub(crate) unsafe fn clear_tile_industry(
    out: *mut IndustryCost,
    tile: TileIndex,
    flags: DoCommandFlags,
    tiles: *const TileServices,
    policy: *const TilePolicyServices,
    costs: *const IndustryCostServices,
    generation: *const super::GenerationServices,
) {
    let (tiles, policy, costs, generation) = unsafe { (&*tiles, &*policy, &*costs, &*generation) };
    let i = unsafe { (tiles.industry)(tile.0) };
    let indspec = get_industry_spec(crate::industry_type::IndustryType(unsafe {
        (*i.owner).metadata.kind
    }));
    let current_company = || CompanyID(unsafe { (policy.current_company)() });
    // water can destroy industries
    // in editor you can bulldoze industries
    // with magic_bulldozer cheat you can destroy industries
    // (area around OILRIG is water, so water shouldn't flood it
    if (current_company() != Owner::WATER
        && !(generation.environment.editor)()
        && !unsafe { (policy.magic_bulldozer)() })
        || flags.test(DoCommandFlag::Auto)
        || (current_company() == Owner::WATER
            && (IndustryBehaviours(unsafe { (*indspec).behaviour })
                .test(IndustryBehaviour::BuiltOnWater)
                || has_bit(
                    unsafe {
                        (*get_industry_tile_spec(tiles.get_industry_gfx(tile))).slopes_refused
                    },
                    5,
                )))
    {
        if flags.test(DoCommandFlag::Auto) {
            unsafe { (policy.object_in_way)(out, indspec) };
        } else {
            unsafe { (costs.error)(out, StringID::INVALID_STRING_ID.0) };
        }
        return;
    }
    if flags.test(DoCommandFlag::Execute) {
        unsafe {
            (policy.close_ai)(i.shell);
            (policy.close_game)(i.shell);
            (policy.destroy)(i.shell);
        }
    }
    let removal = unsafe { super::removal_cost((*indspec).removal_cost_multiplier) };
    unsafe { (costs.expense_money)(out, ExpensesType::EXPENSES_CONSTRUCTION.0, removal) };
}
/// The tables `TerraformTile_Industry` reaches.
pub(crate) struct TerraformServices<'a> {
    pub shared: &'a crate::services::Services,
    pub tiles: &'a TileServices,
    pub policy: &'a TilePolicyServices,
    pub costs: &'a IndustryCostServices,
}
/// `TerraformTile_Industry`.
///
/// # Safety
/// Native complete cost sink and live tile. The resolver can reenter; no owner or
/// map reference survives it. `LandscapeClear` returns its original full cost.
pub(crate) unsafe fn terraform_tile_industry(
    out: *mut IndustryCost,
    tile: TileIndex,
    flags: DoCommandFlags,
    z_new: i32,
    tileh_new: Slope,
    w: &TerraformServices<'_>,
) {
    let (shared, t, p, c) = (w.shared, w.tiles, w.policy, w.costs);
    if (shared.autoslope)() {
        // We imitate here TTDP's behaviour:
        //  - Both new and old slope must not be steep.
        //  - TileMaxZ must not be changed.
        //  - Allow autoslope by default.
        //  - Disallow autoslope if callback succeeds and returns non-zero.
        let tileh_old = Slope(crate::map_func::World::new(shared).tile_slope(tile.0));
        // TileMaxZ must not be changed. Slopes must not be steep.
        if !is_steep_slope(tileh_old.0)
            && !is_steep_slope(tileh_new.0)
            && unsafe { (p.tile_max_z)(tile.0) } == z_new.wrapping_add(get_slope_max_z(tileh_new.0))
        {
            let gfx = t.get_industry_gfx(tile);
            let itspec: *const IndustryTileSpec = get_industry_tile_spec(gfx);
            // Call callback 3C 'disable autosloping for industry tiles'.
            let allow = if unsafe {
                IndustryTileCallbackMasks((*itspec).callback_mask)
                    .test(IndustryTileCallbackMask::Autoslope)
            } {
                // If the callback fails, allow autoslope.
                let res = t.tile_callback(tile, gfx, CallbackID::CBID_INDTILE_AUTOSLOPE);
                res == CALLBACK_FAILED
                    || !(shared.grf_boolean_callback)(
                        unsafe { (*itspec).grf_prop.grffile },
                        CallbackID::CBID_INDTILE_AUTOSLOPE.0,
                        res,
                    )
            } else {
                // Allow autoslope.
                true
            };
            if allow {
                let foundation = crate::company::price(usize::from(Price::PR_BUILD_FOUNDATION.0));
                unsafe {
                    (c.expense_money)(out, ExpensesType::EXPENSES_CONSTRUCTION.0, foundation);
                };
                return;
            }
        }
    }
    crate::landscape_cmd::clear_command(unsafe { &*p.command_checks }, out.cast(), tile.0, flags.0);
}
