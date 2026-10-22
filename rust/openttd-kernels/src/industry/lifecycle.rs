/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Industry destruction and tile/parent trigger traversal. The owner is resolved
//! once, with field-sized raw access only. Every owner/registry borrow ends before
//! callbacks; native station deletion, cargo invalidation and resolver calls can
//! revisit Rust. Map and pool storage stay native. Panics and native exceptions
//! cannot unwind through this synchronous boundary.
use super::{Builder, Industry, Location, Map, Services};
use std::{ffi::c_void, ptr};

#[repr(C)]
pub struct CleanupTile {
    pub industry: u16,
    pub kind: u8,
    pub oilrig: bool,
}
#[repr(C)]
pub struct Reseed {
    pub own: u32,
    pub parent: u32,
    pub used: u8,
}
#[repr(C)]
pub struct RandomisationSpec {
    pub gfx: u16,
    pub enabled: bool,
}
#[repr(C)]
pub struct AnimationSpec {
    pub spec: *const c_void,
    pub triggers: u8,
}
#[repr(C)]
pub struct LifecycleServices {
    pub neutral_station: unsafe extern "C" fn(*mut c_void) -> *mut super::StationHandle,
    pub cleanup_tile: unsafe extern "C" fn(u32) -> CleanupTile,
    pub delete_tile_inspect: unsafe extern "C" fn(u32),
    pub restoration: *const crate::water::RestorationServices,
    pub delete_oilrig: unsafe extern "C" fn(u32),
    pub clear_docking: unsafe extern "C" fn(u32),
    pub industry_field: unsafe extern "C" fn(u32, u16) -> bool,
    pub disassociate_field: unsafe extern "C" fn(u32),
    pub release_disasters: unsafe extern "C" fn(u16),
    pub delete_storage: unsafe extern "C" fn(*mut c_void),
    pub delete_news: unsafe extern "C" fn(u16),
    pub close_view: unsafe extern "C" fn(u16),
    pub close_production: unsafe extern "C" fn(u16),
    pub delete_inspect: unsafe extern "C" fn(u16),
    pub delete_subsidies: unsafe extern "C" fn(u16),
    pub invalidate_cargo: unsafe extern "C" fn(u16),
    pub next_near_station: unsafe extern "C" fn(*mut c_void, *mut c_void) -> *mut c_void,
    pub remove_delivery: unsafe extern "C" fn(*mut c_void, *mut c_void),
    pub belongs: unsafe extern "C" fn(u32, u16) -> bool,
    pub randomisation_spec: unsafe extern "C" fn(u32) -> RandomisationSpec,
    pub random_triggers: unsafe extern "C" fn(u32) -> u8,
    pub set_random_triggers: unsafe extern "C" fn(u32, u8),
    pub resolve_reseed: unsafe extern "C" fn(u32, u16, *mut c_void, u8) -> Reseed,
    pub random_bits: unsafe extern "C" fn(u32) -> u8,
    pub set_random_bits: unsafe extern "C" fn(u32, u8),
    pub animation_spec: unsafe extern "C" fn(u32) -> AnimationSpec,
    pub change_frame: unsafe extern "C" fn(*const c_void, u32, u32, u32),
}

pub(super) unsafe fn location(owner: *mut Industry) -> Location {
    unsafe {
        Location {
            tile: ptr::read(ptr::addr_of!((*owner).metadata.tile)),
            width: ptr::read(ptr::addr_of!((*owner).metadata.width)),
            height: ptr::read(ptr::addr_of!((*owner).metadata.height)),
        }
    }
}
pub(super) fn tiles(area: Location, size_x: u32, mut operation: impl FnMut(u32)) {
    for y in 0..u32::from(area.height) {
        let row = area.tile.wrapping_add(y.wrapping_mul(size_x));
        for x in 0..u32::from(area.width) {
            operation(row.wrapping_add(x));
        }
    }
}

/// # Safety
/// Live shell/owner in its native destructor, including partial initialization or
/// pool cleanup. Tables/map are valid for the call. No callback destroys this owner.
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub unsafe extern "C" fn openttd_rust_industry_cleanup(
    industry: *mut c_void,
    owner: *mut Industry,
    builder: *mut Builder,
    id: u16,
    cleaning: bool,
    map: Map,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    if cleaning {
        return;
    }
    let width = unsafe { ptr::read(ptr::addr_of!((*owner).metadata.width)) };
    if width == 0 {
        return;
    }
    let has_neutral_station = !unsafe { (w.neutral_station)(industry) }.is_null();
    let area = unsafe {
        Location {
            tile: ptr::read(ptr::addr_of!((*owner).metadata.tile)),
            width,
            height: ptr::read(ptr::addr_of!((*owner).metadata.height)),
        }
    };
    tiles(area, map.size_x, |tile| unsafe {
        let observation = (w.cleanup_tile)(tile);
        if observation.kind == 8 {
            if observation.industry == id {
                (w.delete_tile_inspect)(tile);
                let shared = s.shared();
                crate::water::restore(
                    tile,
                    16,
                    shared,
                    &*(shared.water_services)(),
                    &*w.restoration,
                );
            }
        } else if observation.kind == 5 && observation.oilrig {
            (w.delete_oilrig)(tile);
        }
    });
    if has_neutral_station {
        tiles(unsafe { location(owner) }, map.size_x, |tile| unsafe {
            (w.clear_docking)(tile);
        });
    }
    if unsafe { (s.behaviour)(industry) } & 1 != 0 {
        let tile = unsafe { ptr::read(ptr::addr_of!((*owner).metadata.tile)) };
        let x = tile % map.size_x;
        let y = tile / map.size_x;
        let sx = x.saturating_sub(21);
        let sy = y.saturating_sub(21);
        let ex = x.wrapping_add(21).min(map.size_x);
        let ey = y.wrapping_add(21).min(map.size_y);
        let fields = Location {
            tile: sy.wrapping_mul(map.size_x).wrapping_add(sx),
            width: (ex - sx) as u16,
            height: (ey - sy) as u16,
        };
        tiles(fields, map.size_x, |tile| unsafe {
            if (w.industry_field)(tile, id) {
                (w.disassociate_field)(tile);
            }
        });
    }
    unsafe {
        (w.release_disasters)(id);
        (w.delete_storage)(industry);
        let kind = ptr::read(ptr::addr_of!((*owner).metadata.kind));
        super::openttd_rust_industry_registry_erase(builder, kind, id);
        (w.delete_news)(id);
        (w.close_view)(id);
        (w.close_production)(id);
        (w.delete_inspect)(id);
        (w.delete_subsidies)(id);
        (w.invalidate_cargo)(id);
    }
    let mut previous = ptr::null_mut();
    loop {
        let station = unsafe { (w.next_near_station)(industry, previous) };
        if station.is_null() {
            break;
        }
        unsafe { (w.remove_delivery)(station, industry) };
        previous = station;
    }
}

unsafe fn randomise_tile(
    tile: u32,
    trigger: u8,
    industry: *mut c_void,
    s: &Services,
    w: &LifecycleServices,
) -> u32 {
    let spec = unsafe { (w.randomisation_spec)(tile) };
    if !spec.enabled {
        return 0;
    }
    let waiting = unsafe { (w.random_triggers)(tile) } | (1_u8 << trigger);
    unsafe { (w.set_random_triggers)(tile, waiting) };
    let masks = unsafe { (w.resolve_reseed)(tile, spec.gfx, industry, waiting) };
    unsafe { (w.set_random_triggers)(tile, waiting & !masks.used) };
    let new_random_bits = s.shared().random() as u8;
    let random_bits = unsafe { (w.random_bits)(tile) };
    let random_bits = (random_bits & !(masks.own as u8)) | (new_random_bits & masks.own as u8);
    unsafe {
        (w.set_random_bits)(tile, random_bits);
        (s.shared().mark_dirty)(tile);
    }
    masks.parent
}
unsafe fn reseed_industry(owner: *mut Industry, reseed: u32, s: &Services) {
    if reseed == 0 || owner.is_null() {
        return;
    }
    let random_bits = s.shared().random() as u16;
    unsafe {
        let field = ptr::addr_of_mut!((*owner).metadata.random);
        let old = ptr::read(field);
        // Historical &= reseed (not &= ~reseed) is the specification.
        ptr::write(field, (old & reseed as u16) | (random_bits & reseed as u16));
    }
}
/// # Safety
/// Live industry owner for this industry tile and immutable tables; serial call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_randomise_tile(
    tile: u32,
    trigger: u8,
    industry: *mut c_void,
    owner: *mut Industry,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    let reseed = unsafe { randomise_tile(tile, trigger, industry, s, w) };
    unsafe { reseed_industry(owner, reseed, s) };
}
/// # Safety
/// Live industry and map with valid original trigger; tables outlive this call.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_randomise(
    industry: *mut c_void,
    owner: *mut Industry,
    id: u16,
    trigger: u8,
    size_x: u32,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    let mut reseed = 0;
    tiles(unsafe { location(owner) }, size_x, |tile| unsafe {
        if (w.belongs)(tile, id) {
            reseed |= randomise_tile(tile, trigger, industry, s, w);
        }
    });
    unsafe { reseed_industry(owner, reseed, s) };
}
unsafe fn trigger_animation(
    tile: u32,
    trigger: u8,
    random: u32,
    extra: u32,
    w: &LifecycleServices,
) -> bool {
    let spec = unsafe { (w.animation_spec)(tile) };
    if spec.triggers & (1_u8 << trigger) == 0 {
        return false;
    }
    unsafe { (w.change_frame)(spec.spec, tile, random, u32::from(trigger) | extra) };
    true
}
/// # Safety
/// Industry tile and valid original trigger; native frame service may reenter.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_animate_tile(
    tile: u32,
    trigger: u8,
    extra: u32,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) -> bool {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    let random = s.shared().random();
    unsafe { trigger_animation(tile, trigger, random, extra, w) }
}
/// # Safety
/// Live industry and map; immutable tables and valid original trigger.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_industry_animate(
    owner: *mut Industry,
    id: u16,
    trigger: u8,
    size_x: u32,
    services: *const Services,
    lifecycle: *const LifecycleServices,
) -> bool {
    let s = unsafe { &*services };
    let w = unsafe { &*lifecycle };
    let mut result = true;
    let mut random = s.shared().random();
    tiles(unsafe { location(owner) }, size_x, |tile| unsafe {
        if (w.belongs)(tile, id) {
            if trigger_animation(tile, trigger, random, 0, w) {
                // SB does not mask its input, so the draw can set high bits too.
                random = (random & 0xffff_0000) | s.shared().random();
            } else {
                result = false;
            }
        }
    });
    result
}
