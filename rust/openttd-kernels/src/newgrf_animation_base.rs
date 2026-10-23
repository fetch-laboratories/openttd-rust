/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! `newgrf_animation_base.h`: one algorithm shared by six typed bindings.
#![allow(
    unsafe_code,
    missing_docs,
    clippy::cast_possible_truncation,
    clippy::too_many_arguments
)]
use crate::{
    animated_tile::{
        Services, openttd_rust_animation_add as add, openttd_rust_animation_delete as delete,
    },
    newgrf_callbacks::CALLBACK_FAILED,
    station::BaseStation,
};
#[repr(C)]
pub struct Info {
    pub speed: u8,
    pub frames: u8,
    pub looping: bool,
    pub speed_callback: bool,
    pub next_callback: bool,
    pub random: bool,
    pub enabled: bool,
}
#[repr(C)]
pub struct Entry<S, O> {
    pub spec: *const S,
    pub object: *mut O,
}
impl<S, O> Copy for Entry<S, O> {}
impl<S, O> Clone for Entry<S, O> {
    fn clone(&self) -> Self {
        *self
    }
}
#[repr(C)]
pub struct Binding<S, O, E> {
    pub entry: extern "C" fn(u32) -> Entry<S, O>,
    pub info: extern "C" fn(*const S) -> Info,
    pub callback: extern "C" fn(u16, u32, u32, *const S, *mut O, u32, E) -> u16,
    pub sound: extern "C" fn(*const S, u8, u32),
    pub version8: extern "C" fn(*const S) -> bool,
    pub speed_error: extern "C" fn(*const S, u16),
    pub speed_callback: u16,
    pub next_callback: u16,
}
pub trait Frame<O> {
    fn get(a: &Services, o: *mut O, t: u32) -> u8;
    fn set(a: &Services, o: *mut O, t: u32, f: u8) -> bool;
}
pub struct TileFrame;
impl<O> Frame<O> for TileFrame {
    fn get(a: &Services, _: *mut O, t: u32) -> u8 {
        unsafe { ((*a.industry_tiles).frame)(t) }
    }
    fn set(a: &Services, _: *mut O, t: u32, f: u8) -> bool {
        if unsafe { ((*a.industry_tiles).frame)(t) } == f {
            return false;
        }
        (a.shared().tile_animation_set_frame)(t, f);
        true
    }
}
pub struct RoadFrame;
impl Frame<BaseStation> for RoadFrame {
    fn get(a: &Services, o: *mut BaseStation, t: u32) -> u8 {
        (a.road_frame)(o, t)
    }
    fn set(a: &Services, o: *mut BaseStation, t: u32, f: u8) -> bool {
        (a.set_road_frame)(o, t, f)
    }
}
fn sound<S, O, E>(a: &Services, b: &Binding<S, O, E>, spec: *const S, tile: u32, cb: u16) {
    let id = ((cb >> 8) & 0x7F) as u8;
    if id != 0 && unsafe { ((*a.industry_tiles).ambient)() } {
        (b.sound)(spec, id, tile);
    }
}
pub fn animate<S, O, E: Default, F: Frame<O>>(
    a: &Services,
    b: &Binding<S, O, E>,
    e: Entry<S, O>,
    tile: u32,
) {
    let info = (b.info)(e.spec);
    if !info.enabled {
        return;
    }
    let mut speed = info.speed;
    if info.speed_callback {
        let cb = (b.callback)(b.speed_callback, 0, 0, e.spec, e.object, tile, E::default());
        if cb != CALLBACK_FAILED {
            if cb >= 0x100 && (b.version8)(e.spec) {
                (b.speed_error)(e.spec, cb);
            }
            speed = (cb as u8).min(16);
        }
    }
    // Property speeds are raw bytes for five bindings. Keep the native word
    // shift masking; a Rust checked shift would introduce an overflow abort.
    if !(a.shared().tick_counter)().is_multiple_of(1_u64.wrapping_shl(u32::from(speed))) {
        return;
    }
    let mut frame = F::get(a, e.object, tile);
    let mut set = false;
    if info.next_callback {
        let random = if info.random { a.shared().random() } else { 0 };
        let cb = (b.callback)(
            b.next_callback,
            random,
            0,
            e.spec,
            e.object,
            tile,
            E::default(),
        );
        if cb != CALLBACK_FAILED {
            set = true;
            match cb as u8 {
                0xFF => delete(tile, false),
                0xFE => set = false,
                f => frame = f,
            }
            sound(a, b, e.spec, tile, cb);
        }
    }
    if !set {
        if frame < info.frames {
            frame = frame.wrapping_add(1);
        } else if frame == info.frames && info.looping {
            frame = 0;
        } else {
            delete(tile, false);
        }
    }
    if F::set(a, e.object, tile, frame) {
        (a.shared().mark_dirty)(tile);
    }
}
pub fn change<S, O, E, F: Frame<O>>(
    a: &Services,
    b: &Binding<S, O, E>,
    e: Entry<S, O>,
    tile: u32,
    cb: u16,
    random: u32,
    trigger: u32,
    extra: E,
) {
    let cb = (b.callback)(cb, random, trigger, e.spec, e.object, tile, extra);
    if cb == CALLBACK_FAILED {
        return;
    }
    match cb as u8 {
        0xFD => {}
        0xFE => add(tile, false),
        0xFF => delete(tile, false),
        f => {
            let changed = F::set(a, e.object, tile, f);
            add(tile, changed);
        }
    }
    sound(a, b, e.spec, tile, cb);
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAnimationInfo" ["probe"] => Info {
        speed, frames, looping, speed_callback, next_callback, random, enabled
    };
}
