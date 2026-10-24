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
    newgrf_callbacks::{
        CALLBACK_FAILED, CBID_AIRPTILE_ANIMATION_NEXT_FRAME, CBID_AIRPTILE_ANIMATION_SPEED,
        CBID_HOUSE_ANIMATION_NEXT_FRAME, CBID_HOUSE_ANIMATION_SPEED,
        CBID_INDTILE_ANIMATION_NEXT_FRAME, CBID_INDTILE_ANIMATION_SPEED,
        CBID_OBJECT_ANIMATION_NEXT_FRAME, CBID_OBJECT_ANIMATION_SPEED,
        CBID_STATION_ANIMATION_NEXT_FRAME, CBID_STATION_ANIMATION_SPEED, CallbackID,
    },
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
pub struct Binding<S, O, E: Callback<S, O>> {
    pub entry: E::NativeEntry,
    pub info: extern "C" fn(*const S) -> Info,
    pub callback: E::Native,
    pub sound: extern "C" fn(*const S, u8, u32),
    pub version8: extern "C" fn(*const S) -> bool,
    pub speed_error: extern "C" fn(*const S, u16, u16),
}

/// Native callbacks remain only for features whose runtime is still native.
pub trait Callback<S, O>: Sized {
    type Native;
    type NativeEntry;
    fn entry(a: &Services, b: &Binding<S, O, Self>, tile: u32) -> Entry<S, O>;
    fn resolve(
        a: &Services,
        b: &Binding<S, O, Self>,
        cb: u16,
        p1: u32,
        p2: u32,
        spec: *const S,
        object: *mut O,
        tile: u32,
        extra: Self,
    ) -> u16;
}
macro_rules! native_callback {
    ($e:ty) => {
        impl<S, O> Callback<S, O> for $e {
            type NativeEntry = extern "C" fn(u32) -> Entry<S, O>;
            fn entry(_: &Services, b: &Binding<S, O, Self>, tile: u32) -> Entry<S, O> {
                (b.entry)(tile)
            }
            type Native = extern "C" fn(u16, u32, u32, *const S, *mut O, u32, Self) -> u16;
            fn resolve(
                _: &Services,
                b: &Binding<S, O, Self>,
                cb: u16,
                p1: u32,
                p2: u32,
                spec: *const S,
                object: *mut O,
                tile: u32,
                extra: Self,
            ) -> u16 {
                (b.callback)(cb, p1, p2, spec, object, tile, extra)
            }
        }
    };
}
native_callback!(i32);
native_callback!(u64);
#[derive(Default)]
pub struct AirportExtra;
impl Callback<crate::animated_tile::AirportSpec, crate::station::Station> for AirportExtra {
    type Native = ();
    type NativeEntry = ();
    fn entry(
        a: &Services,
        _: &Binding<crate::animated_tile::AirportSpec, crate::station::Station, Self>,
        tile: u32,
    ) -> Entry<crate::animated_tile::AirportSpec, crate::station::Station> {
        let spec = crate::newgrf_airport::tile_spec_by_tile(tile, unsafe { &*a.airport_runtime });
        let station = (a.as_station)((a.station_base)((a.shared().station_index)(tile)));
        Entry {
            spec,
            object: station,
        }
    }
    fn resolve(
        a: &Services,
        _: &Binding<crate::animated_tile::AirportSpec, crate::station::Station, Self>,
        cb: u16,
        p1: u32,
        p2: u32,
        spec: *const crate::animated_tile::AirportSpec,
        object: *mut crate::station::Station,
        tile: u32,
        _: Self,
    ) -> u16 {
        crate::newgrf_airport::tile_callback(cb, p1, p2, spec, object, tile, unsafe {
            &*a.airport_runtime
        })
    }
}

/// The feature fixes the real native carrier types and shared callback IDs.
pub trait Feature {
    type Spec;
    type Owner;
    type Extra: Default + Callback<Self::Spec, Self::Owner>;
    type Frame: Frame<Self::Owner>;
    const SPEED: CallbackID;
    const NEXT: CallbackID;
    fn entry(
        a: &Services,
        b: &Binding<Self::Spec, Self::Owner, Self::Extra>,
        tile: u32,
    ) -> Entry<Self::Spec, Self::Owner> {
        Self::Extra::entry(a, b, tile)
    }
}
macro_rules! feature {
    ($name:ident, $spec:ty, $owner:ty, $extra:ty, $frame:ty, $speed:ident, $next:ident) => {
        pub struct $name;
        impl Feature for $name {
            type Spec = $spec;
            type Owner = $owner;
            type Extra = $extra;
            type Frame = $frame;
            const SPEED: CallbackID = $speed;
            const NEXT: CallbackID = $next;
        }
    };
}
feature!(
    House,
    crate::animated_tile::HouseSpec,
    crate::services::TownHandle,
    u64,
    TileFrame,
    CBID_HOUSE_ANIMATION_SPEED,
    CBID_HOUSE_ANIMATION_NEXT_FRAME
);
feature!(
    Industry,
    crate::industry::TileSpec,
    crate::industry::ShellHandle,
    i32,
    TileFrame,
    CBID_INDTILE_ANIMATION_SPEED,
    CBID_INDTILE_ANIMATION_NEXT_FRAME
);
feature!(
    Station,
    crate::station_cmd::RailSpec,
    BaseStation,
    i32,
    TileFrame,
    CBID_STATION_ANIMATION_SPEED,
    CBID_STATION_ANIMATION_NEXT_FRAME
);
feature!(
    Airport,
    crate::animated_tile::AirportSpec,
    crate::station::Station,
    AirportExtra,
    TileFrame,
    CBID_AIRPTILE_ANIMATION_SPEED,
    CBID_AIRPTILE_ANIMATION_NEXT_FRAME
);
feature!(
    Object,
    crate::objects::Spec,
    crate::objects::Object,
    i32,
    TileFrame,
    CBID_OBJECT_ANIMATION_SPEED,
    CBID_OBJECT_ANIMATION_NEXT_FRAME
);
feature!(
    RoadStop,
    crate::station_cmd::RoadSpec,
    BaseStation,
    i32,
    RoadFrame,
    CBID_STATION_ANIMATION_SPEED,
    CBID_STATION_ANIMATION_NEXT_FRAME
);
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
fn sound<S, O, E: Callback<S, O>>(
    a: &Services,
    b: &Binding<S, O, E>,
    spec: *const S,
    tile: u32,
    cb: u16,
) {
    let id = ((cb >> 8) & 0x7F) as u8;
    if id != 0 && unsafe { ((*a.industry_tiles).ambient)() } {
        (b.sound)(spec, id, tile);
    }
}
pub fn animate<F: Feature>(
    a: &Services,
    b: &Binding<F::Spec, F::Owner, F::Extra>,
    e: Entry<F::Spec, F::Owner>,
    tile: u32,
) {
    let info = (b.info)(e.spec);
    if !info.enabled {
        return;
    }
    let mut speed = info.speed;
    if info.speed_callback {
        let cb = F::Extra::resolve(
            a,
            b,
            F::SPEED.0,
            0,
            0,
            e.spec,
            e.object,
            tile,
            F::Extra::default(),
        );
        if cb != CALLBACK_FAILED {
            if cb >= 0x100 && (b.version8)(e.spec) {
                (b.speed_error)(e.spec, F::SPEED.0, cb);
            }
            speed = (cb as u8).min(16);
        }
    }
    // Raw speeds >=64 match the observed pinned Linux shift masking only;
    // the C++ expression is undefined there. Other targets remain unproved (#156).
    if !(a.shared().tick_counter)().is_multiple_of(1_u64.wrapping_shl(u32::from(speed))) {
        return;
    }
    let mut frame = F::Frame::get(a, e.object, tile);
    let mut set = false;
    if info.next_callback {
        let random = if info.random { a.shared().random() } else { 0 };
        let cb = F::Extra::resolve(
            a,
            b,
            F::NEXT.0,
            random,
            0,
            e.spec,
            e.object,
            tile,
            F::Extra::default(),
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
    if F::Frame::set(a, e.object, tile, frame) {
        (a.shared().mark_dirty)(tile);
    }
}
pub fn change<F: Feature>(
    a: &Services,
    b: &Binding<F::Spec, F::Owner, F::Extra>,
    e: Entry<F::Spec, F::Owner>,
    tile: u32,
    cb: u16,
    random: u32,
    trigger: u32,
    extra: F::Extra,
) {
    let cb = F::Extra::resolve(a, b, cb, random, trigger, e.spec, e.object, tile, extra);
    if cb == CALLBACK_FAILED {
        return;
    }
    match cb as u8 {
        0xFD => {}
        0xFE => add(tile, false),
        0xFF => delete(tile, false),
        f => {
            let changed = F::Frame::set(a, e.object, tile, f);
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
