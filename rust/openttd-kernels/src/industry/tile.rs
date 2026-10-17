/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry tile construction, builtin animation and tile-loop control.
//! Map writes and resolver callbacks stay synchronous and can reenter. Owner
//! fields are read through short raw accesses, never borrowed across callbacks.
// Retain the original tick masks for source comparison.
#![allow(clippy::verbose_bit_mask)]
use super::{
    ConstructionServices, GenerationServices, Industry, IndustryCost, IndustryCostServices,
    LifecycleServices, Services, SpecHandle,
};
use std::{ffi::c_void, ptr};

#[repr(C)]
pub struct EffectHandle {
    _private: [u8; 0],
}
#[repr(C)]
pub struct ShellHandle {
    _private: [u8; 0],
}
#[repr(C)]
pub struct TileIndustry {
    pub shell: *mut ShellHandle,
    pub owner: *mut Industry,
}
#[repr(C)]
pub struct TileSpec {
    _private: [u8; 0],
}
#[repr(C)]
pub struct TileAcceptance {
    pub cargo: [u8; 16],
    pub amount: [i8; 16],
    pub accepts_all: bool,
    pub accepts_callback: bool,
    pub amount_callback: bool,
}
#[repr(C)]
pub struct TileServices {
    pub gfx: unsafe extern "C" fn(u32) -> u16,
    pub set_gfx: unsafe extern "C" fn(u32, u16),
    pub frame: unsafe extern "C" fn(u32) -> u8,
    pub set_frame: unsafe extern "C" fn(u32, u8),
    pub animation_loop: unsafe extern "C" fn(u32) -> u8,
    pub set_animation_loop: unsafe extern "C" fn(u32, u8),
    pub ambient: unsafe extern "C" fn() -> bool,
    pub animate_new: unsafe extern "C" fn(u32),
    pub construction_counter: unsafe extern "C" fn(u32) -> u8,
    pub construction_stage: unsafe extern "C" fn(u32) -> u8,
    pub completed: unsafe extern "C" fn(u32) -> bool,
    pub set_completed: unsafe extern "C" fn(u32),
    pub reset_construction: unsafe extern "C" fn(u32),
    pub industry: unsafe extern "C" fn(u32) -> TileIndustry,
    pub industry_index: unsafe extern "C" fn(u32) -> u16,
    pub toffee_offset: unsafe extern "C" fn(u8) -> u8,
    pub on_water: unsafe extern "C" fn(u32) -> bool,
    pub water_loop: unsafe extern "C" fn(u32),
    pub transport: unsafe extern "C" fn(u32) -> bool,
    pub anim_production: unsafe extern "C" fn(u16) -> u16,
    pub anim_next: unsafe extern "C" fn(u16) -> u16,
    pub build_oilrig: unsafe extern "C" fn(u32),
    pub pixel_max_z: unsafe extern "C" fn(u32) -> i32,
    pub create_effect: unsafe extern "C" fn(i32, i32, i32, u8) -> *mut EffectHandle,
    pub create_effect_above: unsafe extern "C" fn(i32, i32, i32, u8) -> *mut EffectHandle,
    pub set_effect_substate: unsafe extern "C" fn(*mut EffectHandle, u8),
    pub spec: unsafe extern "C" fn(u16) -> *const TileSpec,
    pub acceptance: unsafe extern "C" fn(*const TileSpec) -> TileAcceptance,
    pub callback: unsafe extern "C" fn(u32, u16, u16, u32, u32) -> u16,
    pub cargo_translation: unsafe extern "C" fn(*const TileSpec, u8) -> u8,
}
fn chance(s: &Services, a: u32, b: u32) -> bool {
    crate::services::chance16_i(a, b, s.shared().random())
}
fn sound(tile: u32, sound: u16, s: &Services, t: &TileServices) {
    if unsafe { (t.ambient)() } {
        unsafe { (s.play_sound)(sound, tile) };
    }
}
fn dirty(tile: u32, s: &Services) {
    (s.shared().mark_dirty)(tile);
}
fn chimney(tile: u32, map: crate::services::Map, t: &TileServices) {
    let x = map.tile_x(tile).wrapping_mul(16);
    let y = map.tile_y(tile).wrapping_mul(16);
    let z = unsafe { (t.pixel_max_z)(tile) };
    unsafe {
        (t.create_effect)(
            x.wrapping_add(15) as i32,
            y.wrapping_add(14) as i32,
            z.wrapping_add(59),
            0,
        )
    };
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
        super::openttd_rust_industry_animate_tile(tile, 0, 0, s, l);
        if stage == 3 {
            (t.set_completed)(tile);
        }
    }
    dirty(tile, s);
    if !unsafe { (t.completed)(tile) } {
        return;
    }
    let gfx = unsafe { (t.gfx)(tile) };
    if gfx >= 175 {
        return;
    }
    match gfx {
        8 => chimney(tile, map, t),
        24 => {
            let other = tile.wrapping_add(map.size_x());
            if (s.shared().tile_type)(other) == 8 && unsafe { (t.gfx)(other) } == 24 {
                let own = unsafe { (t.industry_index)(tile) };
                let other = unsafe { (t.industry_index)(other) };
                if own == other {
                    unsafe { (t.build_oilrig)(tile) };
                }
            }
        }
        143 | 162 | 165 => unsafe {
            (t.set_frame)(tile, 0);
            (t.set_animation_loop)(tile, 0);
        },
        148..=155 => unsafe { (c.add_animated)(tile, false) },
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
                sound(tile, 9, s, t);
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
    if unsafe { (c.animation_enabled)(gfx) } {
        unsafe { (t.animate_new)(tile) };
        return;
    }
    match gfx {
        174 if (s.shared().tick_counter)() & 1 == 0 => {
            let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1);
            if unsafe { (t.ambient)() } {
                match frame & 7 {
                    2 => unsafe { (s.play_sound)(45, tile) },
                    6 => unsafe { (s.play_sound)(41, tile) },
                    _ => {}
                }
            }
            if frame >= 96 {
                frame = 0;
                (s.shared().delete_animated)(tile);
            }
            unsafe { (t.set_frame)(tile, frame) };
            dirty(tile, s);
        }
        165 if (s.shared().tick_counter)() & 3 == 0 => {
            let mut frame = unsafe { (t.frame)(tile) };
            if unsafe { (t.toffee_offset)(frame) } == 0xff {
                sound(tile, 48, s, t);
            }
            frame = frame.wrapping_add(1);
            if frame >= 70 {
                frame = 0;
                (s.shared().delete_animated)(tile);
            }
            unsafe { (t.set_frame)(tile, frame) };
            dirty(tile, s);
        }
        162 if (s.shared().tick_counter)() & 1 == 0 => {
            let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1);
            if frame >= 40 {
                frame = 0;
                (s.shared().delete_animated)(tile);
            }
            unsafe { (t.set_frame)(tile, frame) };
            dirty(tile, s);
        }
        10 if (s.shared().tick_counter)() & 3 == 0 => {
            let frame = unsafe { (t.frame)(tile) };
            if frame == 6 {
                unsafe {
                    (t.set_frame)(tile, 0);
                    (s.shared().delete_animated)(tile);
                }
            } else {
                unsafe { (t.set_frame)(tile, frame.wrapping_add(1)) };
            }
            dirty(tile, s);
        }
        143 if (s.shared().tick_counter)() & 1 == 0 => {
            let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1);
            match frame {
                1 => sound(tile, 44, s, t),
                23 => sound(tile, 43, s, t),
                28 => sound(tile, 42, s, t),
                _ => {
                    if frame >= 50 {
                        let mut count =
                            u32::from(unsafe { (t.animation_loop)(tile) }).wrapping_add(1);
                        frame = 0;
                        if count >= 8 {
                            count = 0;
                            (s.shared().delete_animated)(tile);
                        }
                        unsafe { (t.set_animation_loop)(tile, count as u8) };
                    }
                }
            }
            unsafe { (t.set_frame)(tile, frame) };
            dirty(tile, s);
        }
        148..=155 if (s.shared().tick_counter)() & 3 == 0 => {
            unsafe { (t.set_gfx)(tile, if gfx < 155 { gfx.wrapping_add(1) } else { 148 }) };
            dirty(tile, s);
        }
        30..=32 if (s.shared().tick_counter)() & 7 == 0 => {
            let stop = chance(s, 1, 7);
            let mut frame = unsafe { (t.frame)(tile) }.wrapping_add(1);
            let mut gfx = gfx;
            let last = if frame == 4 {
                frame = 0;
                gfx = gfx.wrapping_add(1);
                if gfx == 33 {
                    gfx = 30;
                    true
                } else {
                    false
                }
            } else {
                false
            };
            if last && stop {
                unsafe {
                    (t.set_gfx)(tile, 29);
                    (c.set_construction_stage)(tile, 3);
                    (s.shared().delete_animated)(tile);
                }
            } else {
                unsafe {
                    (t.set_frame)(tile, frame);
                    (t.set_gfx)(tile, gfx);
                }
            }
            dirty(tile, s);
        }
        1 | 48 | 88 => mine(tile, s, t),
        _ => {}
    }
}
fn bubble(tile: u32, s: &Services, map: crate::services::Map, t: &TileServices) {
    sound(tile, 46, s, t);
    let dir = (s.shared().random() & 3) as usize;
    let x = map
        .tile_x(tile)
        .wrapping_mul(16)
        .wrapping_add([11_i32, 0, -4, -14][dir] as u32) as i32;
    let y = map
        .tile_y(tile)
        .wrapping_mul(16)
        .wrapping_add([-4_i32, -10, -4, 1][dir] as u32) as i32;
    let vehicle = unsafe { (t.create_effect_above)(x, y, [49, 59, 60, 65][dir], 9) };
    if !vehicle.is_null() {
        unsafe { (t.set_effect_substate)(vehicle, dir as u8) };
    }
}
/// # Safety
/// Live map tile. Water can destroy its industry, so no owner is resolved until
/// the source's post-water type recheck. Later callbacks retain no Rust borrow.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_loop(
    tile: u32,
    log_x: u8,
    s: *const Services,
    c: *const ConstructionServices,
    l: *const LifecycleServices,
    t: *const TileServices,
    g: *const GenerationServices,
) {
    let (s, c, l, t) = unsafe { (&*s, &*c, &*l, &*t) };
    let map = crate::services::Map { log_x };
    if unsafe { (t.on_water)(tile) } {
        unsafe { (t.water_loop)(tile) };
    }
    if (s.shared().tile_type)(tile) != 8 {
        return;
    }
    let industry = unsafe { (t.industry)(tile) };
    unsafe {
        super::openttd_rust_industry_randomise_tile(
            tile,
            0,
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
    if unsafe { ((*g).editor)() } {
        return;
    }
    if unsafe { (t.transport)(tile) } {
        let industry = unsafe { (t.industry)(tile) };
        if !unsafe {
            super::openttd_rust_industry_animate(
                industry.owner,
                (t.industry_index)(tile),
                4,
                map.size_x(),
                s,
                l,
            )
        } {
            let gfx = unsafe { (t.gfx)(tile) };
            let next = unsafe { (t.anim_production)(gfx) };
            if next != 255 {
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
    if unsafe { super::openttd_rust_industry_animate_tile(tile, 1, 0, s, l) } {
        return;
    }
    let gfx = unsafe { (t.gfx)(tile) };
    let next = unsafe { (t.anim_next)(gfx) };
    if next != 255 {
        unsafe {
            (t.reset_construction)(tile);
            (t.set_gfx)(tile, next);
        };
        dirty(tile, s);
        return;
    }
    let mut gfx = unsafe { (t.gfx)(tile) };
    match gfx {
        0 | 47 | 79 => {
            if (s.shared().tick_counter)() & 0x400 == 0 && chance(s, 1, 2) {
                gfx = match gfx {
                    0 => 1,
                    47 => 48,
                    _ => 88,
                };
                unsafe {
                    (t.set_gfx)(tile, gfx);
                    (t.set_frame)(tile, 0x80);
                    (c.add_animated)(tile, true);
                }
            }
        }
        29 => {
            if chance(s, 1, 6) {
                unsafe {
                    (t.set_gfx)(tile, 30);
                    (t.set_frame)(tile, 0);
                    (c.add_animated)(tile, true);
                }
            }
        }
        1 | 48 | 88 => {
            if (s.shared().tick_counter)() & 0x400 == 0 {
                gfx = match gfx {
                    1 => 0,
                    48 => 47,
                    _ => 79,
                };
                unsafe {
                    (t.set_gfx)(tile, gfx);
                    (t.set_completed)(tile);
                    (c.set_construction_stage)(tile, 3);
                    (s.shared().delete_animated)(tile);
                }
                dirty(tile, s);
            }
        }
        10 => {
            if chance(s, 1, 3) {
                sound(tile, 10, s, t);
                unsafe { (c.add_animated)(tile, true) };
            }
        }
        49 => {
            unsafe {
                (t.create_effect_above)(
                    map.tile_x(tile).wrapping_mul(16).wrapping_add(6) as i32,
                    map.tile_y(tile).wrapping_mul(16).wrapping_add(6) as i32,
                    43,
                    11,
                )
            };
        }
        143 => {
            let industry = unsafe { (t.industry)(tile) };
            let delivered =
                unsafe { ptr::read(ptr::addr_of!((*industry.owner).fields.was_cargo_delivered)) };
            if delivered != 0 {
                unsafe {
                    (*industry.owner).fields.was_cargo_delivered = 0;
                    (t.set_animation_loop)(tile, 0);
                    (c.add_animated)(tile, true);
                }
            }
        }
        161 => bubble(tile, s, map, t),
        165 => unsafe { (c.add_animated)(tile, true) },
        174 if chance(s, 1, 3) => {
            unsafe { (c.add_animated)(tile, true) };
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
    let spec = unsafe { (t.spec)(gfx) };
    let industry = unsafe { (t.industry)(tile) };
    let mut info = unsafe { (t.acceptance)(spec) };
    if info.accepts_all {
        for n in 0..super::accepted_count(industry.owner) {
            let cargo = unsafe { (*super::accepted(industry.owner, n)).cargo };
            let Some(slot) = info
                .cargo
                .iter()
                .position(|&c| c == cargo)
                .or_else(|| info.cargo.iter().position(|&c| c == 255))
            else {
                continue;
            };
            info.cargo[slot] = cargo;
            info.amount[slot] = info.amount[slot].wrapping_add(8);
        }
    }
    if info.accepts_callback {
        let result = unsafe { (t.callback)(tile, gfx, 0x2c, 0, 0) };
        if result != 0xffff {
            info.cargo.fill(255);
            for n in 0..3 {
                info.cargo[n] =
                    unsafe { (t.cargo_translation)(spec, ((result >> (n * 5)) & 31) as u8) };
            }
        }
    }
    if info.amount_callback {
        let result = unsafe { (t.callback)(tile, gfx, 0x2b, 0, 0) };
        if result != 0xffff {
            info.amount.fill(0);
            for n in 0..3 {
                info.amount[n] = ((result >> (n * 4)) & 15) as i8;
            }
        }
    }
    for n in 0..16 {
        let cargo = info.cargo[n];
        let amount = info.amount[n];
        if cargo == 255 || amount <= 0 {
            continue;
        }
        unsafe {
            let entry = acceptance.add(usize::from(cargo));
            *entry = (*entry).wrapping_add(amount as u32);
        }
        let bit = 1_u64.wrapping_shl(u32::from(cargo));
        if unsafe { *always_accepted } & bit != 0 {
            continue;
        }
        if (0..super::accepted_count(industry.owner))
            .any(|slot| unsafe { (*super::accepted(industry.owner, slot)).cargo == cargo })
        {
            continue;
        }
        unsafe { *always_accepted |= bit };
    }
}

#[repr(C)]
pub struct TilePolicyServices {
    pub current_company: unsafe extern "C" fn() -> u8,
    pub magic_bulldozer: unsafe extern "C" fn() -> bool,
    pub object_in_way: unsafe extern "C" fn(*mut IndustryCost, *const SpecHandle),
    pub slopes_refused: unsafe extern "C" fn(*const TileSpec) -> u8,
    pub close_ai: unsafe extern "C" fn(*mut ShellHandle),
    pub close_game: unsafe extern "C" fn(*mut ShellHandle),
    pub destroy: unsafe extern "C" fn(*mut ShellHandle),
    pub removal_cost: unsafe extern "C" fn(*const SpecHandle) -> i64,
    pub tile_max_z: unsafe extern "C" fn(u32) -> i32,
    pub autoslope_callback: unsafe extern "C" fn(*const TileSpec) -> bool,
    pub callback_boolean: unsafe extern "C" fn(*const TileSpec, u16, u16) -> bool,
    pub landscape_clear: unsafe extern "C" fn(*mut IndustryCost, u32, u16),
}
/// # Safety
/// Live tile and native stack result. Event and destruction callbacks may reenter;
/// immutable specs remain live after owner destruction, with no owner borrow held.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_tile_clear(
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
    let spec = unsafe { (g.spec)(kind) };
    if (unsafe { (p.current_company)() } != 17
        && !unsafe { (g.editor)() }
        && !unsafe { (p.magic_bulldozer)() })
        || flags & 2 != 0
        || (unsafe { (p.current_company)() } == 17
            && (unsafe { (g.spec_behaviour)(spec) } & 4 != 0
                || unsafe { (p.slopes_refused)((t.spec)((t.gfx)(tile))) } & 32 != 0))
    {
        if flags & 2 != 0 {
            unsafe { (p.object_in_way)(out, spec) };
        } else {
            unsafe { (c.error)(out, 0xffff) };
        }
        return;
    }
    if flags & 1 != 0 {
        unsafe {
            (p.close_ai)(industry.shell);
            (p.close_game)(industry.shell);
            (p.destroy)(industry.shell);
        }
    }
    let removal = unsafe { (p.removal_cost)(spec) };
    unsafe { (c.expense_money)(out, 0, removal) };
}
/// # Safety
/// Native complete cost sink and live tile. The resolver can reenter; no owner or
/// map reference survives it. `LandscapeClear` returns its original full cost.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_tile_terraform(
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
        let old_slope = (shared.tile_slope)(tile);
        let max_z = if slope_new == 0 {
            0
        } else if slope_new & 16 != 0 {
            2
        } else {
            1
        };
        if old_slope & 16 == 0
            && slope_new & 16 == 0
            && unsafe { (p.tile_max_z)(tile) } == z_new.wrapping_add(max_z)
        {
            let gfx = unsafe { (t.gfx)(tile) };
            let spec = unsafe { (t.spec)(gfx) };
            let allowed = if unsafe { (p.autoslope_callback)(spec) } {
                let result = unsafe { (t.callback)(tile, gfx, 0x3c, 0, 0) };
                result == 0xffff || !unsafe { (p.callback_boolean)(spec, 0x3c, result) }
            } else {
                true
            };
            if allowed {
                let foundation = (shared.foundation_price)();
                unsafe { (c.expense_money)(out, 0, foundation) };
                return;
            }
        }
    }
    unsafe { (p.landscape_clear)(out, tile, flags) };
}
