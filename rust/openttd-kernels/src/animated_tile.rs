/*
 * This file is part of OpenTTD.
 * OpenTTD is free software; you can redistribute it and/or modify it under the terms of the GNU General Public License as published by the Free Software Foundation, version 2.
 * OpenTTD is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.
 * See the GNU General Public License for more details. You should have received a copy of the GNU General Public License along with OpenTTD. If not, see <https://www.gnu.org/licenses/old-licenses/gpl-2.0>.
 */

//! Process-wide animated-tile queue, dispatch and animation-trigger traversal.
//! The game-state mutex serializes GUI/game threads. Queue accesses end before
//! every callback. Actual scheduled animation callbacks only change frames,
//! sounds and deferred deletion; they cannot allocate/erase queue entries.
//! Specs/pool shells stay live across resolver calls: GRF evaluation neither
//! resets specs/map nor yields or destroys the animated shell.
#![allow(unsafe_code, missing_docs, clippy::cast_possible_truncation)]
use crate::animated_tile_map::AnimatedTileState;
use crate::cargo_type::{INVALID_CARGO, NUM_CARGO};
use crate::industry::{ShellHandle as IndustryShell, TileSpec as IndustrySpec};
use crate::newgrf_animation_base::{Binding, Entry};
use crate::newgrf_callbacks::{
    CBID_AIRPTILE_ANIMATION_TRIGGER, CBID_INDTILE_ANIMATION_TRIGGER, CBID_OBJECT_ANIMATION_TRIGGER,
    CBID_STATION_ANIMATION_TRIGGER,
};
use crate::station_cmd::{RailSpec, RoadSpec};
use crate::station_type::{
    AAT_ACCEPTANCE_TICK, AAT_BUILT, SAT_ACCEPTANCE_TICK, SAT_CARGO_TAKEN, SAT_NEW_CARGO,
};
use crate::{
    industry::{ConstructionServices, Services as Industries, TileServices},
    objects::{Object, Spec as ObjectSpec},
    services::{Services as Shared, TownHandle},
    station::{BaseStation, Station},
    tile_type::{INVALID_TILE, MP_HOUSE, MP_INDUSTRY, MP_OBJECT, MP_STATION},
    town::Leaves,
};
use std::cell::UnsafeCell;
#[repr(C)]
pub struct HouseSpec {
    _private: [u8; 0],
}
#[repr(C)]
pub struct AirportSpec {
    _private: [u8; 0],
}
#[repr(C)]
#[derive(Clone, Copy)]
pub struct Area {
    pub tile: u32,
    pub width: u16,
    pub height: u16,
}
#[repr(C)]
pub struct Services {
    pub shared: *const Shared,
    pub station_base: extern "C" fn(u16) -> *mut BaseStation,
    pub as_station: extern "C" fn(*mut BaseStation) -> *mut Station,
    pub location: extern "C" fn(*const BaseStation) -> u32,
    pub is_station: extern "C" fn(*mut BaseStation) -> bool,
    pub rail_randomisation: extern "C" fn(*mut BaseStation, u32, u8, u8),
    pub roadstop_resolver: *const crate::newgrf_roadstop::Services,
    pub town: *const Leaves,
    pub industries: *const Industries,
    pub construction: *const ConstructionServices,
    pub industry_tiles: *const TileServices,
    pub house: *const Binding<HouseSpec, TownHandle, u64>,
    pub industry: *const Binding<IndustrySpec, IndustryShell, i32>,
    pub station: *const Binding<RailSpec, BaseStation, i32>,
    pub airport: *const Binding<AirportSpec, Station, i32>,
    pub object: *const Binding<ObjectSpec, Object, i32>,
    pub roadstop: *const Binding<RoadSpec, BaseStation, i32>,
    pub state: extern "C" fn(u32) -> AnimatedTileState,
    pub set_state: extern "C" fn(u32, AnimatedTileState),
    pub house_spec: extern "C" fn(u16) -> *const HouseSpec,
    pub road_frame: extern "C" fn(*mut BaseStation, u32) -> u8,
    pub set_road_frame: extern "C" fn(*mut BaseStation, u32, u8) -> bool,
    pub rail_area: extern "C" fn(*mut BaseStation, u32, u8) -> Area,
    pub rail_belongs: extern "C" fn(*mut BaseStation, u32) -> bool,
    pub station_triggers: extern "C" fn(*mut BaseStation, u8) -> bool,
    pub road_triggers: extern "C" fn(*mut BaseStation, u8) -> bool,
    pub rail_spec: extern "C" fn(u32) -> *const RailSpec,
    pub airport_spec: extern "C" fn(u32) -> *const AirportSpec,
    pub industry_object: extern "C" fn(u32) -> *mut IndustryShell,
    pub rail_spec_triggers: extern "C" fn(*const RailSpec) -> u16,
    pub road_spec_triggers: extern "C" fn(*const RoadSpec) -> u16,
    pub airport_spec_triggers: extern "C" fn(*const AirportSpec) -> u8,
    pub rail_cargo: extern "C" fn(*const RailSpec, u8) -> u8,
    pub road_cargo: extern "C" fn(*const RoadSpec, u8) -> u8,
    pub airport_cargo: extern "C" fn(*const AirportSpec, u8) -> u8,
    pub airport_grf: extern "C" fn(*const AirportSpec) -> bool,
    pub airport_area: extern "C" fn(*mut Station) -> Area,
    pub airport_belongs: extern "C" fn(*const Station, u32) -> bool,
    pub road_count: extern "C" fn(*mut BaseStation) -> usize,
    pub road_tile: extern "C" fn(*mut BaseStation, usize) -> u32,
}
struct Registry(UnsafeCell<*const Services>);
// SAFETY: installation and all calls are serialized by the game-state mutex.
unsafe impl Sync for Registry {}
static REGISTRY: Registry = Registry(UnsafeCell::new(std::ptr::null()));
/// Install the immutable native table at `InitializeAnimatedTiles`, before play.
/// Native fixtures temporarily install a stack table and restore its predecessor.
/// No owner borrow spans installation; the previous table is never dereferenced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn openttd_rust_animation_exchange(
    table: *const Services,
) -> *const Services {
    unsafe {
        let previous = *REGISTRY.0.get();
        *REGISTRY.0.get() = table;
        previous
    }
}
pub(crate) fn services<'a>() -> &'a Services {
    // SAFETY: InitializeAnimatedTiles installs the process-lifetime native table
    // before any map use. A native test scope pins its replacement until restore.
    unsafe { &**REGISTRY.0.get() }
}

impl Services {
    pub fn shared(&self) -> &Shared {
        unsafe { &*self.shared }
    }
}
struct Queue(UnsafeCell<Vec<u32>>);
// SAFETY: all entries run under the native game-state mutex (including GUI
// commands). No queue reference escapes its callback-free lexical access.
unsafe impl Sync for Queue {}
static QUEUE: Queue = Queue(UnsafeCell::new(Vec::new()));
fn queue<R>(f: impl FnOnce(&mut Vec<u32>) -> R) -> R {
    unsafe { f(&mut *QUEUE.0.get()) }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animation_count() -> usize {
    queue(|q| q.len())
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animation_at(i: usize) -> u32 {
    queue(|q| q[i])
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animation_append(t: u32) {
    queue(|q| q.push(t));
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animation_initialize() {
    queue(Vec::clear);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animation_add(tile: u32, dirty: bool) {
    let a = services();
    let s = a.shared();
    if dirty {
        (s.mark_dirty)(tile);
    }
    let state = (a.state)(tile);
    if state == AnimatedTileState::ANIMATED {
        return;
    }
    if state == AnimatedTileState::NONE {
        queue(|q| q.push(tile));
    }
    (a.set_state)(tile, AnimatedTileState::ANIMATED);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animation_delete(tile: u32, immediate: bool) {
    let a = services();
    if immediate {
        if (a.state)(tile) == AnimatedTileState::NONE {
            return;
        }
        (a.set_state)(tile, AnimatedTileState::NONE);
        queue(|q| {
            if let Some(i) = q.iter().position(|t| *t == tile) {
                q.swap_remove(i);
            }
        });
    } else if (a.state)(tile) == AnimatedTileState::ANIMATED {
        (a.set_state)(tile, AnimatedTileState::DELETED);
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animation_run() {
    let a = services();
    let mut i = 0;
    while i < openttd_rust_animation_count() {
        let tile = openttd_rust_animation_at(i);
        if (a.state)(tile) != AnimatedTileState::ANIMATED {
            (a.set_state)(tile, AnimatedTileState::NONE);
            queue(|q| {
                q.swap_remove(i);
            });
            continue;
        }
        openttd_rust_animate_tile(tile);
        i += 1;
    }
}
pub(crate) fn may_animate(tile: u32, shared: &Shared) -> bool {
    matches!(
        (shared.tile_type)(tile),
        MP_HOUSE | MP_STATION | MP_INDUSTRY | MP_OBJECT
    )
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_may_animate(tile: u32) -> bool {
    may_animate(tile, services().shared())
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animate_tile(tile: u32) {
    let a = services();
    let s = a.shared();
    match (s.tile_type)(tile) {
        MP_HOUSE => unsafe {
            crate::town::house::openttd_rust_house_animate(a.town, a.shared, tile);
        },
        MP_INDUSTRY => unsafe {
            crate::industry::openttd_rust_industry_tile_animate(
                tile,
                a.industries,
                a.construction,
                a.industry_tiles,
            );
        },
        MP_OBJECT => openttd_rust_animate_object(tile),
        MP_STATION => {
            if (unsafe { &*(*((s.movement_services)())).rail_map }.has_station_rail)(tile) {
                openttd_rust_animate_station(tile);
            } else if crate::station_map::is_airport(tile, s) {
                openttd_rust_animate_airport(tile);
            } else if (s.is_any_road_stop)(tile) {
                openttd_rust_animate_roadstop(tile);
            }
        }
        _ => crate::movement::native_assert(s, false),
    }
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_animation_afterload(clean: bool, flags: bool) {
    let a = services();
    if clean {
        let mut i = 0;
        while i < openttd_rust_animation_count() {
            let tile = openttd_rust_animation_at(i);
            let remove = !openttd_rust_may_animate(tile) || queue(|q| q[..i].contains(&tile));
            if remove {
                queue(|q| {
                    q.remove(i);
                });
            } else {
                i += 1;
            }
        }
    }
    if flags {
        for t in 0..(1_u32 << (a.shared().map_log_x)()) * ((a.shared().map_max_y)() + 1) {
            if openttd_rust_may_animate(t) {
                (a.set_state)(t, AnimatedTileState::NONE);
            }
        }
        for i in 0..openttd_rust_animation_count() {
            let t = openttd_rust_animation_at(i);
            if t != INVALID_TILE {
                (a.set_state)(t, AnimatedTileState::ANIMATED);
            }
        }
    }
}
macro_rules! animate {
    ($fn:ident,$field:ident,$feature:ty) => {
        #[unsafe(no_mangle)]
        pub extern "C" fn $fn(tile: u32) {
            let a = services();
            let b = unsafe { &*a.$field };
            let e = (b.entry)(tile);
            if !e.spec.is_null() {
                crate::newgrf_animation_base::animate::<$feature>(a, b, e, tile);
            }
        }
    };
}
animate!(
    openttd_rust_animate_house,
    house,
    crate::newgrf_animation_base::House
);
animate!(
    openttd_rust_animate_industry,
    industry,
    crate::newgrf_animation_base::Industry
);
animate!(
    openttd_rust_animate_station,
    station,
    crate::newgrf_animation_base::Station
);
animate!(
    openttd_rust_animate_airport,
    airport,
    crate::newgrf_animation_base::Airport
);
animate!(
    openttd_rust_animate_object,
    object,
    crate::newgrf_animation_base::Object
);
animate!(
    openttd_rust_animate_roadstop,
    roadstop,
    crate::newgrf_animation_base::RoadStop
);
pub(crate) unsafe fn house_change(
    cb: u16,
    house: u16,
    obj: *mut TownHandle,
    tile: u32,
    random: u32,
    trigger: u32,
    extra: u64,
) {
    let a = services();
    crate::newgrf_animation_base::change::<crate::newgrf_animation_base::House>(
        a,
        unsafe { &*a.house },
        Entry {
            spec: (a.house_spec)(house),
            object: obj,
        },
        tile,
        cb,
        random,
        trigger,
        extra,
    );
}
pub(crate) unsafe fn industry_change(
    spec: *const std::ffi::c_void,
    tile: u32,
    random: u32,
    trigger: u32,
) {
    let a = services();
    let b = unsafe { &*a.industry };
    let object = (a.industry_object)(tile);
    crate::newgrf_animation_base::change::<crate::newgrf_animation_base::Industry>(
        a,
        b,
        Entry {
            spec: spec.cast(),
            object,
        },
        tile,
        CBID_INDTILE_ANIMATION_TRIGGER.0,
        random,
        trigger,
        0,
    );
}
pub(crate) unsafe fn object_change(
    spec: *const ObjectSpec,
    obj: *mut Object,
    tile: u32,
    random: u32,
    trigger: u32,
) {
    let a = services();
    crate::newgrf_animation_base::change::<crate::newgrf_animation_base::Object>(
        a,
        unsafe { &*a.object },
        Entry { spec, object: obj },
        tile,
        CBID_OBJECT_ANIMATION_TRIGGER.0,
        random,
        trigger,
        0,
    );
}
fn tiles(area: Area, log_x: u8, f: impl FnMut(u32)) {
    crate::tilearea::TileArea {
        tile: area.tile,
        w: area.width,
        h: area.height,
    }
    .tiles(crate::map_func::Map { log_x })
    .for_each(f);
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_trigger_station(
    st: *mut BaseStation,
    tile: u32,
    trigger: u8,
    cargo: u8,
) {
    let a = services();
    if !(a.station_triggers)(st, trigger) {
        return;
    }
    let random = a.shared().random() as u16;
    let areas = [0, 2, 2, 1, 1, 1, 2, 0, 1];
    let area = (a.rail_area)(st, tile, areas[usize::from(trigger)]);
    let b = unsafe { &*a.station };
    tiles(area, (a.shared().map_log_x)() as u8, |t| {
        if !(a.rail_belongs)(st, t) {
            return;
        }
        let spec = (a.rail_spec)(t);
        if !spec.is_null() && (a.rail_spec_triggers)(spec) & (1 << trigger) != 0 {
            let extra = if cargo < NUM_CARGO {
                (u16::from((a.rail_cargo)(spec, cargo)) << 8) as u8
            } else {
                0
            };
            crate::newgrf_animation_base::change::<crate::newgrf_animation_base::Station>(
                a,
                b,
                Entry { spec, object: st },
                t,
                CBID_STATION_ANIMATION_TRIGGER.0,
                (u32::from(random) << 16) | (a.shared().random() & 0xFFFF),
                u32::from(trigger) | u32::from(extra),
                0,
            );
        }
    });
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_trigger_roadstop(
    st: *mut BaseStation,
    tile: u32,
    trigger: u8,
    cargo: u8,
) {
    let a = services();
    if !(a.road_triggers)(st, trigger) {
        return;
    }
    let random = a.shared().random() as u16;
    let b = unsafe { &*a.roadstop };
    let process = |t| {
        let spec =
            unsafe { crate::newgrf_roadstop::openttd_rust_roadstop_spec(t, a.roadstop_resolver) };
        if !spec.is_null() && (a.road_spec_triggers)(spec) & (1 << trigger) != 0 {
            let extra = if cargo < NUM_CARGO {
                (u16::from((a.road_cargo)(spec, cargo)) << 8) as u8
            } else {
                0
            };
            crate::newgrf_animation_base::change::<crate::newgrf_animation_base::RoadStop>(
                a,
                b,
                Entry { spec, object: st },
                t,
                CBID_STATION_ANIMATION_TRIGGER.0,
                (u32::from(random) << 16) | (a.shared().random() & 0xFFFF),
                u32::from(trigger) | u32::from(extra),
                0,
            );
        }
    };
    if matches!(
        trigger,
        SAT_NEW_CARGO | SAT_CARGO_TAKEN | SAT_ACCEPTANCE_TICK
    ) {
        for i in 0..(a.road_count)(st) {
            process((a.road_tile)(st, i));
        }
    } else {
        process(tile);
    }
}
fn airport_trigger(
    a: &Services,
    st: *mut Station,
    tile: u32,
    trigger: u8,
    random: u32,
    extra: u8,
) -> bool {
    let b = unsafe { &*a.airport };
    let spec = (a.airport_spec)(tile);
    if (a.airport_spec_triggers)(spec) & (1 << trigger) == 0 {
        return false;
    }
    crate::newgrf_animation_base::change::<crate::newgrf_animation_base::Airport>(
        a,
        b,
        Entry { spec, object: st },
        tile,
        CBID_AIRPTILE_ANIMATION_TRIGGER.0,
        random,
        u32::from(trigger) | u32::from(extra),
        0,
    );
    true
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_trigger_airport_tile(
    st: *mut Station,
    tile: u32,
    trigger: u8,
) -> bool {
    let a = services();
    airport_trigger(a, st, tile, trigger, a.shared().random(), 0)
}
#[unsafe(no_mangle)]
pub extern "C" fn openttd_rust_trigger_airport(st: *mut Station, trigger: u8, cargo: u8) -> bool {
    let a = services();
    let area = (a.airport_area)(st);
    if area.tile == INVALID_TILE {
        return false;
    }
    let mut ret = true;
    let mut random = a.shared().random();
    tiles(area, (a.shared().map_log_x)() as u8, |t| {
        if !(a.airport_belongs)(st, t) {
            return;
        }
        let spec = (a.airport_spec)(t);
        if !(a.airport_grf)(spec) {
            return;
        }
        let extra = if cargo < NUM_CARGO {
            (u16::from((a.airport_cargo)(spec, cargo)) << 8) as u8
        } else {
            0
        };
        if airport_trigger(a, st, t, trigger, random, extra) {
            crate::bitmath_func::sb(&mut random, 0, 16, a.shared().random());
        } else {
            ret = false;
        }
    });
    ret
}

crate::abi::layouts! {
    fn abi_layout;
    "OpenTTDAnimationServices" ["probe"] => Services {
        shared,
        station_base, as_station, location, is_station, rail_randomisation, roadstop_resolver,
        town,
        industries,
        construction,
        industry_tiles,
        house,
        industry,
        station,
        airport,
        object,
        roadstop,
        state,
        set_state,
        house_spec,
        road_frame,
        set_road_frame,
        rail_area,
        rail_belongs,
        station_triggers,
        road_triggers,
        rail_spec, airport_spec, industry_object,
        rail_spec_triggers,
        road_spec_triggers,
        airport_spec_triggers,
        rail_cargo,
        road_cargo,
        airport_cargo,
        airport_grf,
        airport_area,
        airport_belongs,
        road_count,
        road_tile,
    };
    "OpenTTDAnimationArea" ["probe"] => Area { tile, width, height };
    "OpenTTDHouseAnimation" ["probe"] => Binding<HouseSpec, TownHandle, u64> { entry, info, callback, sound, version8, speed_error };
    "OpenTTDIndustryAnimation" ["probe"] => Binding<IndustrySpec, IndustryShell, i32> { entry, info, callback, sound, version8, speed_error };
    "OpenTTDStationAnimation" ["probe"] => Binding<RailSpec, BaseStation, i32> { entry, info, callback, sound, version8, speed_error };
    "OpenTTDAirportAnimation" ["probe"] => Binding<AirportSpec, Station, i32> { entry, info, callback, sound, version8, speed_error };
    "OpenTTDObjectAnimation" ["probe"] => Binding<ObjectSpec, Object, i32> { entry, info, callback, sound, version8, speed_error };
    "OpenTTDRoadStopAnimation" ["probe"] => Binding<RoadSpec, BaseStation, i32> { entry, info, callback, sound, version8, speed_error };
    "OpenTTDHouseAnimationEntry" ["probe"] => Entry<HouseSpec, TownHandle> { spec, object };
    "OpenTTDIndustryAnimationEntry" ["probe"] => Entry<IndustrySpec, IndustryShell> { spec, object };
    "OpenTTDStationAnimationEntry" ["probe"] => Entry<RailSpec, BaseStation> { spec, object };
    "OpenTTDAirportAnimationEntry" ["probe"] => Entry<AirportSpec, Station> { spec, object };
    "OpenTTDObjectAnimationEntry" ["probe"] => Entry<ObjectSpec, Object> { spec, object };
    "OpenTTDRoadStopAnimationEntry" ["probe"] => Entry<RoadSpec, BaseStation> { spec, object };
}

/// Native station lookup keeps the source's strict `Station::Get` operation and
/// performs its `BaseStation` conversion before crossing the boundary.
pub(crate) fn station_base(id: u16) -> *mut BaseStation {
    (services().station_base)(id)
}
pub(crate) fn rail_event(st: *mut BaseStation, tile: u32, trigger: u8, random_trigger: u8) {
    (services().rail_randomisation)(st, tile, random_trigger, INVALID_CARGO);
    openttd_rust_trigger_station(st, tile, trigger, INVALID_CARGO);
}
pub(crate) fn road_event(st: *mut BaseStation, tile: u32, trigger: u8, random_trigger: u8) {
    unsafe {
        crate::newgrf_roadstop::openttd_rust_roadstop_randomise(
            st,
            tile,
            random_trigger,
            INVALID_CARGO,
            services().roadstop_resolver,
        );
    }
    openttd_rust_trigger_roadstop(st, tile, trigger, INVALID_CARGO);
}
pub(crate) fn cargo_event(
    st: *mut BaseStation,
    cargo: u8,
    trigger: u8,
    airport_trigger: u8,
    random_trigger: u8,
) {
    let a = services();
    let tile = (a.location)(st);
    (a.rail_randomisation)(st, tile, random_trigger, cargo);
    openttd_rust_trigger_station(st, tile, trigger, cargo);
    openttd_rust_trigger_airport((a.as_station)(st), airport_trigger, cargo);
    unsafe {
        crate::newgrf_roadstop::openttd_rust_roadstop_randomise(
            st,
            tile,
            random_trigger,
            cargo,
            a.roadstop_resolver,
        );
    }
    openttd_rust_trigger_roadstop(st, tile, trigger, cargo);
}
pub(crate) fn acceptance(st: *mut BaseStation) {
    let a = services();
    let tile = (a.location)(st);
    openttd_rust_trigger_station(st, tile, SAT_ACCEPTANCE_TICK, INVALID_CARGO);
    openttd_rust_trigger_roadstop(st, tile, SAT_ACCEPTANCE_TICK, INVALID_CARGO);
    if (a.is_station)(st) {
        openttd_rust_trigger_airport((a.as_station)(st), AAT_ACCEPTANCE_TICK, INVALID_CARGO);
    }
}
pub(crate) fn airport_built(st: *mut BaseStation, tile: u32) {
    openttd_rust_trigger_airport_tile((services().as_station)(st), tile, AAT_BUILT);
}

/// Borrow the immutable road-stop leaf table installed before animation/gameplay.
pub(crate) fn roadstop_resolver_services() -> *const crate::newgrf_roadstop::Services {
    services().roadstop_resolver
}
